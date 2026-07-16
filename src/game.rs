use std::fmt;
use std::ops::{Index, Sub};

use crate::action::{Action, IntoAction};
use crate::piece::{Color, Piece, Role};
use crate::position::Position;
use crate::reduce::{DrawReason, Ending, Mode, Rejected, mode, reduce};
use crate::san::{to_figurine, to_san};
use crate::square::Square;

/// A game is its starting position plus the log of accepted actions —
/// the log is the canonical record (it is morally a PGN); replay is a fold,
/// undo is dropping the last entry, variations share a prefix.
///
/// `history` is the memoized fold: every intermediate position the log has
/// produced, with `history[ply]` the position after `ply` plies (so
/// `history[0]` is the start). It is a cache — `Game::replay` over the log
/// always reproduces it.
///
/// `mode` is memoized the same way: deriving it costs a pass over the legal
/// actions, so it is computed once per accepted move. While `Playing`,
/// applying an action checks only the move; once `Played`, no move checking
/// is done at all — every action is `Rejected::GameOver` in O(1).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Game {
    start: Position,
    log: Vec<Action>,
    history: Vec<Position>,
    mode: Mode,
}

impl Default for Game {
    fn default() -> Game {
        Game::from_position(Position::default())
    }
}

impl Game {
    pub fn new() -> Game {
        Game::default()
    }

    pub fn from_position(start: Position) -> Game {
        Game {
            start,
            log: Vec::new(),
            history: vec![start],
            mode: mode(start),
        }
    }

    /// Pure: returns a new game, this one is untouched.
    pub fn apply(&self, action: impl IntoAction) -> Result<Game, Rejected> {
        if let Mode::Played(ending) = self.mode {
            return Err(Rejected::GameOver { ending });
        }
        let action = action.into_action(self.position())?;
        let next = reduce(self.position(), action)?;
        let mut log = self.log.clone();
        let mut history = self.history.clone();
        log.push(action);
        history.push(next);
        let mut game = Game {
            start: self.start,
            log,
            history,
            mode: mode(next),
        };
        // The automatic draws live above the position: fivefold and
        // seventy-five-move are facts about the history, arriving by
        // themselves the way mate does.
        if game.mode == Mode::Playing {
            if game.repetitions() >= 5 {
                game.mode = Mode::Played(Ending::Draw(DrawReason::Fivefold));
            } else if game.quiet_plies() >= 150 {
                game.mode = Mode::Played(Ending::Draw(DrawReason::SeventyFiveMoves));
            }
        }
        Ok(game)
    }

    pub fn position(&self) -> Position {
        *self.history.last().expect("history always holds the start")
    }

    pub fn log(&self) -> &[Action] {
        &self.log
    }

    /// Playing, or played — and if played, how it ended. Memoized: this is
    /// a field read, not a derivation.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// Plies since the last capture or pawn move — the fifty-move rule's
    /// clock. Derived from the log, never stored: FEN's halfmove counter,
    /// computed on demand.
    pub fn quiet_plies(&self) -> usize {
        self.log
            .iter()
            .enumerate()
            .rev()
            .take_while(|&(ply, &action)| {
                let before = self.history[ply];
                let (from, to) = match action {
                    Action::Move { from, to } | Action::Promote { from, to, .. } => (from, to),
                };
                let pawn = before
                    .at(from)
                    .is_some_and(|piece| piece.role == Role::Pawn);
                !pawn && before.at(to).is_none()
            })
            .count()
    }

    /// How many times the current position has occurred, counted by FIDE's
    /// "same position" ([`Position::repetition_key`]) — at least 1, since
    /// the current position counts itself.
    pub fn repetitions(&self) -> usize {
        let key = self.position().repetition_key();
        self.history
            .iter()
            .filter(|past| past.repetition_key() == key)
            .count()
    }

    /// The capture tray: every piece taken so far, in the order they left
    /// the board. There is no capture *list* — captured pieces simply
    /// cease to exist when the interpreter lifts them — but the log knows:
    /// each ply's victim is whatever stood on the struck square in the
    /// position before the move. Derived, never stored.
    pub fn captures(&self) -> Vec<Piece> {
        self.log
            .iter()
            .enumerate()
            .filter_map(|(ply, &action)| victim(self.history[ply], action))
            .collect()
    }

    /// Claim the draw the position has armed — threefold repetition or the
    /// fifty-move rule. Like an unnoticed flag, an unclaimed draw keeps
    /// the game playing; the automatic draws (fivefold, seventy-five,
    /// insufficient material) never need asking.
    pub fn claim_draw(&self) -> Result<Game, Rejected> {
        if let Mode::Played(ending) = self.mode {
            return Err(Rejected::GameOver { ending });
        }
        let reason = if self.repetitions() >= 3 {
            DrawReason::Threefold
        } else if self.quiet_plies() >= 100 {
            DrawReason::FiftyMoves
        } else {
            return Err(Rejected::NoDrawToClaim);
        };
        Ok(Game {
            mode: Mode::Played(Ending::Draw(reason)),
            ..self.clone()
        })
    }

    /// Number of plies played. A *ply* is a half-move — one action by one
    /// player — the term game theory uses (layers of the game tree, like
    /// plies of plywood) because chess's "move" confusingly means a pair:
    /// `1. e4 e5` is one move but two plies. The ply is this crate's
    /// natural unit, since the fold steps one action at a time and every
    /// ply gets its own entry in the history. White makes the odd plies
    /// (1st, 3rd, …), Black the even — `self[ply].turn()` says who moves
    /// next — and a publication's move number is just `ply / 2 + 1`,
    /// which is exactly what [`Game::score`] prints. A game that ends in
    /// "17 moves" by White's mate is 33 plies: Black never answered.
    pub fn plies(&self) -> usize {
        self.log.len()
    }

    /// The position after `ply` plies, or `None` past either end of the
    /// game. `position_at(0)` is the starting position; symbolic indices
    /// work too: `position_at(Terminus - 1)`.
    pub fn position_at(&self, ply: impl Into<Ply>) -> Option<Position> {
        let resolved = ply.into().resolve(self.plies())?;
        self.history.get(resolved).copied()
    }

    /// Replay a log over a starting position: a game is a fold.
    pub fn replay(
        start: Position,
        log: impl IntoIterator<Item = Action>,
    ) -> Result<Position, Rejected> {
        log.into_iter().try_fold(start, reduce)
    }

    /// The current position as FEN with the true counters: the halfmove
    /// clock derived from the log ([`Game::quiet_plies`]) and the
    /// fullmove number from the ply count — both history, neither stored.
    pub fn fen(&self) -> String {
        self.position()
            .fen_with(self.quiet_plies(), self.plies() / 2 + 1)
    }

    /// The game as a publication would print it:
    /// `1. e4 e5 2. Nf3 d6 … 17. Rd8# 1-0`. Also what `Display` shows.
    pub fn score(&self) -> String {
        self.scored(to_san)
    }

    /// The score in figurine algebraic notation, each move wearing its
    /// mover's glyph: `17. ♖d8# 1-0`.
    pub fn figurines(&self) -> String {
        self.scored(to_figurine)
    }

    fn scored(&self, write: fn(Position, Action) -> Result<String, Rejected>) -> String {
        let mut out = String::new();
        for (ply, &action) in self.log.iter().enumerate() {
            let position = self.history[ply];
            if position.turn() == Color::White {
                out.push_str(&format!("{}. ", ply / 2 + 1));
            }
            out.push_str(&write(position, action).expect("the log holds only accepted actions"));
            out.push(' ');
        }
        out.push_str(self.mode.result().unwrap_or("*"));
        out
    }

    /// A new game without the last action. With the fold memoized this is
    /// just dropping the last cache entry — no replay. The mode needs no
    /// recomputation either: the position we return to once accepted a
    /// move, which is proof it was `Playing`.
    pub fn undo(&self) -> Game {
        if self.log.is_empty() {
            return self.clone();
        }
        let plies = self.plies() - 1;
        Game {
            start: self.start,
            log: self.log[..plies].to_vec(),
            history: self.history[..plies + 1].to_vec(),
            mode: Mode::Playing,
        }
    }
}

/// What `action` removed from the board of `before`: the piece on the
/// struck square, or — for the diagonal pawn move onto an empty square —
/// the pawn that just passed beside it.
fn victim(before: Position, action: Action) -> Option<Piece> {
    let (from, to) = match action {
        Action::Move { from, to } | Action::Promote { from, to, .. } => (from, to),
    };
    if let Some(piece) = before.at(to) {
        return Some(piece);
    }
    let mover = before.at(from)?;
    if mover.role == Role::Pawn && from.file() != to.file() {
        let passed = Square::new(to.file(), from.rank())?;
        return before.at(passed);
    }
    None
}

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.score())
    }
}

/// A timeline index: either an absolute ply count or a distance back from
/// the end of the game. Mostly built implicitly — a `usize` is a
/// `Ply::Number`, and [`Terminus`]` - n` is a `Ply::FromEnd(n)`.
/// (A ply is a half-move; see [`Game::plies`] for the full etymology.)
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Ply {
    Number(usize),
    FromEnd(usize),
}

impl Ply {
    fn resolve(self, plies: usize) -> Option<usize> {
        match self {
            Ply::Number(number) => (number <= plies).then_some(number),
            Ply::FromEnd(back) => plies.checked_sub(back),
        }
    }
}

impl From<usize> for Ply {
    fn from(number: usize) -> Ply {
        Ply::Number(number)
    }
}

/// The end of the game as an index: `game[Terminus]` is the final position,
/// `game[Terminus - 1]` the position just before the last ply.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Terminus;

impl From<Terminus> for Ply {
    fn from(_: Terminus) -> Ply {
        Ply::FromEnd(0)
    }
}

impl Sub<usize> for Terminus {
    type Output = Ply;

    fn sub(self, back: usize) -> Ply {
        Ply::FromEnd(back)
    }
}

impl Sub<usize> for Ply {
    type Output = Ply;

    fn sub(self, back: usize) -> Ply {
        match self {
            Ply::Number(number) => Ply::Number(number - back),
            Ply::FromEnd(already) => Ply::FromEnd(already + back),
        }
    }
}

/// Jump notation: `game[ply]` is the position after `ply` plies — `game[0]`
/// is the start, `game[game.plies()]` and `game[Terminus]` the current
/// position, `game[Terminus - 1]` the position before the last ply.
/// Panics past either end of the game, like any slice; use
/// [`Game::position_at`] for the checked form.
impl<P: Into<Ply>> Index<P> for Game {
    type Output = Position;

    fn index(&self, ply: P) -> &Position {
        let ply = ply.into();
        let resolved = ply
            .resolve(self.plies())
            .unwrap_or_else(|| panic!("{ply:?} is outside a {}-ply game", self.plies()));
        &self.history[resolved]
    }
}
