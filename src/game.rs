use std::ops::{Index, Sub};

use crate::action::{Action, IntoAction};
use crate::position::Position;
use crate::reduce::{Mode, Rejected, mode, reduce};

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
        Game { start, log: Vec::new(), history: vec![start], mode: mode(start) }
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
        Ok(Game { start: self.start, log, history, mode: mode(next) })
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

    /// Number of plies played. White makes the odd plies (1st, 3rd, …),
    /// Black the even — `self[ply].turn()` says who moves next.
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

/// A timeline index: either an absolute ply count or a distance back from
/// the end of the game. Mostly built implicitly — a `usize` is a
/// `Ply::Number`, and [`Terminus`]` - n` is a `Ply::FromEnd(n)`.
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
