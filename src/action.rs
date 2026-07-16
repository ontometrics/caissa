use std::str::FromStr;

use crate::piece::Role;
use crate::position::Position;
use crate::reduce::Rejected;
use crate::san::San;
use crate::square::Square;

/// A player's intent, and nothing more. The position supplies the rest:
/// what stands on `from`, whether `to` is a capture.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Action {
    /// Move whatever stands on `from` to `to`.
    Move { from: Square, to: Square },
    /// A pawn reaching the last rank must say what it becomes.
    Promote {
        from: Square,
        to: Square,
        into: Role,
    },
}

impl From<(Square, Square)> for Action {
    fn from((from, to): (Square, Square)) -> Action {
        Action::Move { from, to }
    }
}

/// UCI-style notation: `"h2h4"`, or `"h7h8q"` for promotion (q/r/b/n).
impl FromStr for Action {
    type Err = Rejected;

    fn from_str(s: &str) -> Result<Action, Rejected> {
        let reject = || Rejected::Unparseable(s.to_string());
        if !(4..=5).contains(&s.len()) {
            return Err(reject());
        }
        let from: Square = s[0..2].parse().map_err(|_| reject())?;
        let to: Square = s[2..4].parse().map_err(|_| reject())?;
        match s.as_bytes().get(4) {
            None => Ok(Action::Move { from, to }),
            Some(b'q') => Ok(Action::Promote {
                from,
                to,
                into: Role::Queen,
            }),
            Some(b'r') => Ok(Action::Promote {
                from,
                to,
                into: Role::Rook,
            }),
            Some(b'b') => Ok(Action::Promote {
                from,
                to,
                into: Role::Bishop,
            }),
            Some(b'n') => Ok(Action::Promote {
                from,
                to,
                into: Role::Knight,
            }),
            Some(_) => Err(reject()),
        }
    }
}

/// Anything that can stand in for an action at a call site, so the public
/// API never requires naming the enum: `position.play("h2h4")`,
/// `game.apply("Nf3")`. The position is threaded through because SAN is a
/// description that only a position can resolve; UCI-shaped inputs ignore
/// it.
pub trait IntoAction {
    fn into_action(self, position: Position) -> Result<Action, Rejected>;
}

impl IntoAction for Action {
    fn into_action(self, _: Position) -> Result<Action, Rejected> {
        Ok(self)
    }
}

impl IntoAction for (Square, Square) {
    fn into_action(self, _: Position) -> Result<Action, Rejected> {
        Ok(self.into())
    }
}

impl IntoAction for San {
    fn into_action(self, position: Position) -> Result<Action, Rejected> {
        self.resolve(position)
    }
}

/// UCI first — its grammar is regular and disjoint from SAN — then SAN.
impl IntoAction for &str {
    fn into_action(self, position: Position) -> Result<Action, Rejected> {
        self.parse::<Action>()
            .or_else(|_| self.parse::<San>()?.resolve(position))
    }
}
