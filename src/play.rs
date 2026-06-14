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
use crate::piece::Color;
use crate::position::Position;
use crate::reduce::Mode;

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
