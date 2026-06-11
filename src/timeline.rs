//! A timestamped game. The chess core stays pure — time is an annotation
//! on the log, generic over the timestamp type `T`, so tests can use plain
//! integers and a real recorder can use [`std::time::Instant`].
//!
//! A replay is a value: [`Timeline::frames`] yields each move with the time
//! it took. Playing it back in realtime — each move held literally as long
//! as the player thought — is one effect at the edge:
//!
//! ```no_run
//! use std::time::Instant;
//! use caissa::{Position, Timeline};
//!
//! # fn render(_: caissa::Position) {}
//! let timeline = Timeline::begin(Position::default(), Instant::now())
//!     .record("e2e4", Instant::now())?
//!     .record("e7e5", Instant::now())?;
//!
//! for frame in timeline.frames() {
//!     std::thread::sleep(frame.think_time);
//!     render(frame.position);
//! }
//! # Ok::<(), caissa::Rejected>(())
//! ```

use std::ops::Sub;

use crate::action::{Action, IntoAction};
use crate::game::Game;
use crate::position::Position;
use crate::reduce::{Mode, Rejected};

/// A [`Game`] whose log is annotated with when each ply was made.
///
/// `started` and `ended` are a Snodgrass-style valid-time interval:
/// `ended` is `None` while the game is being played, and closes with the
/// stamp of the move that finished it. It is the chronological shadow of
/// [`Mode`] — the game's logic decides *that* it ended (and how, and for
/// whom); the timeline merely records *when*.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Timeline<T> {
    started: T,
    ended: Option<T>,
    game: Game,
    stamps: Vec<T>,
}

/// One step of a replay: what was played, what it produced, and how long
/// the player took. `D` is whatever subtracting two timestamps yields —
/// [`std::time::Duration`] for `Instant`, the integer itself for integers.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Frame<D> {
    pub think_time: D,
    pub action: Action,
    pub position: Position,
}

fn ended_stamp<T>(game: &Game, at: T) -> Option<T> {
    match game.mode() {
        Mode::Played(_) => Some(at),
        Mode::Playing => None,
    }
}

impl<T: Copy + PartialOrd> Timeline<T> {
    pub fn begin(start: Position, at: T) -> Timeline<T> {
        let game = Game::from_position(start);
        // A game born from a terminal position was over before we began
        // watching: an empty interval.
        let ended = ended_stamp(&game, at);
        Timeline { started: at, ended, game, stamps: Vec::new() }
    }

    /// Pure: returns a new timeline, this one is untouched. The action goes
    /// through [`reduce`](crate::reduce) like any other — time never changes
    /// what is legal, it only records. The move that finishes the game
    /// closes the valid-time interval with its own stamp.
    pub fn record(&self, action: impl IntoAction, at: T) -> Result<Timeline<T>, Rejected> {
        let previous = *self.stamps.last().unwrap_or(&self.started);
        if at < previous {
            return Err(Rejected::OutOfOrder);
        }
        let game = self.game.apply(action)?;
        let ended = ended_stamp(&game, at);
        let mut stamps = self.stamps.clone();
        stamps.push(at);
        Ok(Timeline { started: self.started, ended, game, stamps })
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn started(&self) -> T {
        self.started
    }

    /// The most recent stamp on the timeline — the last move's, or
    /// `started` before any move.
    pub fn latest(&self) -> T {
        *self.stamps.last().unwrap_or(&self.started)
    }

    /// Snodgrass-style: `None` means the game is still being played; once
    /// played, the stamp of the move that ended it.
    pub fn ended(&self) -> Option<T> {
        self.ended
    }

    /// The replay as a value: every ply with the time it took, in order.
    pub fn frames(&self) -> impl Iterator<Item = Frame<T::Output>> + '_
    where
        T: Sub,
    {
        self.stamps.iter().enumerate().map(|(ply, &at)| {
            let before = if ply == 0 { self.started } else { self.stamps[ply - 1] };
            Frame {
                think_time: at - before,
                action: self.game.log()[ply],
                position: self.game[ply + 1],
            }
        })
    }
}
