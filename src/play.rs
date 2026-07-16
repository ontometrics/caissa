//! Self-play, headless: a bot is a pure function of state.
//!
//! A [`Player`] reacts to the game — it needs the state, never a screen —
//! so [`between`] is just a loop: ask the side to move for an action, fold
//! it, repeat until the game is played, and out comes a [`Game`]. No
//! rendering, no I/O. That is what lets it be the corpus flywheel for the
//! learning horizons (millions of games, no view in the way); a human who
//! wants to watch replays the produced game through
//! [`Timeline`](crate::Timeline) afterward — rendering is a separate,
//! optional consumer.
//!
//! Randomness is a *parameter* — the seed — the way time is a parameter
//! to `Timeline`, so a self-play game is a reproducible value: same seed,
//! same game. The crate stays dependency-free; the PRNG is a few lines.
//!
//! ```
//! use caissa::Position;
//! use caissa::play::{between, Random};
//!
//! // a full game, headless and reproducible
//! let game = between(&Random::seeded(1), &Random::seeded(2), Position::default());
//! # let _ = game.score();
//! ```

use crate::action::Action;
use crate::game::Game;
use crate::piece::{Color, Role};
use crate::position::Position;
use crate::reduce::{Ending, Mode};
use crate::square::Square;

/// A bot: it chooses an action for the side to move. It is handed the
/// whole `Game` — the position, plus the sliver of history the non-Markov
/// rules (repetition, the fifty-move clock) would need — but a simple bot
/// reads only `game.position()`. Called only while the game is playing,
/// so a legal action always exists.
pub trait Player {
    fn choose(&self, game: &Game) -> Action;
}

/// Play a game between two bots from `start`, headless. The loop asks the
/// side to move for an action and folds it until the game is played; the
/// rules guarantee termination (the seventy-five-move and fivefold draws
/// cap every line). The result is a `Game` — score it, export it, replay
/// it, or tally it into a dictionary.
pub fn between(white: &impl Player, black: &impl Player, start: Position) -> Game {
    let mut game = Game::from_position(start);
    while game.mode() == Mode::Playing {
        let action = match game.position().turn() {
            Color::White => white.choose(&game),
            Color::Black => black.choose(&game),
        };
        game = game.apply(action).expect("a player returns a legal action");
    }
    game
}

/// A player that picks a legal action uniformly at random, seeded so the
/// game is reproducible: same seed, same game. Vary the seed to generate
/// a varied corpus.
pub struct Random {
    seed: u64,
}

impl Random {
    pub fn seeded(seed: u64) -> Random {
        Random { seed }
    }
}

impl Player for Random {
    fn choose(&self, game: &Game) -> Action {
        let actions: Vec<Action> = game.position().legal_actions().collect();
        let roll = splitmix64(self.seed ^ game.plies() as u64);
        actions[roll as usize % actions.len()]
    }
}

/// A tiny deterministic mixer (splitmix64) — keeps the crate
/// dependency-free while making self-play reproducible.
fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A bot that searches: it runs the fold speculatively to `depth` plies,
/// assuming the opponent answers best, and keeps the line that scores
/// highest under `eval`. This is the Bellman fixed point approximated to
/// a horizon — minimax is greedy on an estimator improved by unrolling,
/// so depth substitutes for evaluator accuracy. `eval` is Ŵ; swap a
/// stronger one in (mobility, structure, a trained net) without touching
/// the search.
pub struct Minimax<E> {
    depth: u32,
    eval: E,
}

impl<E: Fn(Position) -> i32> Minimax<E> {
    /// Search `depth` plies deep, scoring leaves with `eval`.
    pub fn new(depth: u32, eval: E) -> Minimax<E> {
        Minimax { depth, eval }
    }
}

impl<E: Fn(Position) -> i32> Player for Minimax<E> {
    fn choose(&self, game: &Game) -> Action {
        let position = game.position();
        position
            .legal_actions()
            .max_by_key(|&action| {
                -negamax(
                    child(position, action),
                    self.depth.saturating_sub(1),
                    1,
                    &self.eval,
                )
            })
            .expect("called only while the game is playing")
    }
}

/// Larger than any material sum, so a forced mate always outweighs
/// material — and offset by `ply` so a *shorter* mate scores higher,
/// which is what makes the engine actually deliver it.
const MATE: i32 = 1_000_000;

/// The negamax value of a position to the side to move, searching `depth`
/// plies with `eval` at the leaves — the position-scoring companion to
/// [`Minimax`] (which chooses) and [`material`] (the leaf eval). Use it to
/// give an annotator or estimator tactical sight: `evaluate(p, 2,
/// &material)` sees two plies of captures and mates that static material
/// misses.
pub fn evaluate(position: Position, depth: u32, eval: &impl Fn(Position) -> i32) -> i32 {
    negamax(position, depth, 0, eval)
}

/// The value of `position` to the side to move, looking `depth` plies
/// ahead. `mode()` is checked first, so a terminal position is scored
/// exactly even at the horizon — the leaf evaluator that never lies.
fn negamax<E: Fn(Position) -> i32>(position: Position, depth: u32, ply: i32, eval: &E) -> i32 {
    match position.mode() {
        Mode::Played(Ending::Checkmate { .. }) => -(MATE - ply), // the side to move is mated
        Mode::Played(_) => 0,                                    // stalemate or draw
        Mode::Playing if depth == 0 => eval(position),
        Mode::Playing => position
            .legal_actions()
            .map(|action| -negamax(child(position, action), depth - 1, ply + 1, eval))
            .max()
            .expect("a playing position has at least one legal action"),
    }
}

/// The position after a known-legal action — search only ever applies
/// moves it enumerated, so this never rejects.
fn child(position: Position, action: Action) -> Position {
    position
        .play(action)
        .expect("a legal action applies cleanly")
}

/// Material balance from the side to move's view, in centipawns — the
/// first rung of the evaluator ladder. The classic 1/3/3/5/9 are early,
/// crude estimates of W; a stronger `eval` is a richer rung.
pub fn material(position: Position) -> i32 {
    let mut score = 0;
    for square in Square::all() {
        if let Some(piece) = position.at(square) {
            let value = match piece.role {
                Role::Pawn => 100,
                Role::Knight => 320,
                Role::Bishop => 330,
                Role::Rook => 500,
                Role::Queen => 900,
                Role::King => 0,
            };
            score += if piece.color == position.turn() {
                value
            } else {
                -value
            };
        }
    }
    score
}
