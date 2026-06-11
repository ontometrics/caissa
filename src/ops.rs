//! Operator notation. `->` is not overloadable in Rust; `>>` is the
//! arrow-shaped operator that is.
//!
//! - `from >> to` builds an [`Action`]
//! - `action >> role` turns it into a promotion
//! - `position + action` applies it (and `Result + action` chains, so a
//!   line of moves threads its errors monadically)

use std::ops::{Add, Shr};

use crate::action::Action;
use crate::piece::Role;
use crate::position::Position;
use crate::reduce::{Rejected, reduce};
use crate::square::Square;

impl Shr<Square> for Square {
    type Output = Action;

    fn shr(self, to: Square) -> Action {
        Action::Move { from: self, to }
    }
}

impl Shr<Role> for Action {
    type Output = Action;

    fn shr(self, into: Role) -> Action {
        let (Action::Move { from, to } | Action::Promote { from, to, .. }) = self;
        Action::Promote { from, to, into }
    }
}

impl Add<Action> for Position {
    type Output = Result<Position, Rejected>;

    fn add(self, action: Action) -> Result<Position, Rejected> {
        reduce(self, action)
    }
}

impl Add<Action> for Result<Position, Rejected> {
    type Output = Result<Position, Rejected>;

    fn add(self, action: Action) -> Result<Position, Rejected> {
        self.and_then(|position| reduce(position, action))
    }
}
