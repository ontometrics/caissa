//! The clock layer: how a `Playing` game forces a move.
//!
//! A pure value can't force anyone — a position just sits there, owed an
//! action. What the core can do is reify the obligation: whose turn it is,
//! how much of their budget they have spent ([`Clocked::spent`] — a fold
//! over the frames, derived rather than stored), and what follows when the
//! budget is exceeded. Expiry enters the game the way everything else
//! does, as an event: [`Clocked::claim_flag`]. The only thing left at the
//! edge is whoever watches the wall clock and makes the claim.
//!
//! `D` is the budget/duration type — whatever subtracting two stamps
//! yields. Each player gets the same budget for the whole game.

use std::ops::{Add, Sub};

use crate::action::IntoAction;
use crate::piece::Color;
use crate::position::Position;
use crate::reduce::{Ending, Mode, Rejected};
use crate::timeline::Timeline;

/// A [`Timeline`] under time control.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Clocked<T, D> {
    timeline: Timeline<T>,
    budget: D,
    /// Winner on time and the stamp of the claim, once a flag falls.
    flagged: Option<(Color, T)>,
}

impl<T, D> Clocked<T, D>
where
    T: Copy + PartialOrd + Sub<Output = D>,
    D: Copy + PartialOrd + Default + Add<Output = D>,
{
    pub fn begin(start: Position, at: T, budget: D) -> Clocked<T, D> {
        Clocked {
            timeline: Timeline::begin(start, at),
            budget,
            flagged: None,
        }
    }

    pub fn timeline(&self) -> &Timeline<T> {
        &self.timeline
    }

    /// Playing, or played — where "played" now includes losing on time.
    pub fn mode(&self) -> Mode {
        match self.flagged {
            Some((winner, _)) => Mode::Played(Ending::Flagged { winner }),
            None => self.timeline.game().mode(),
        }
    }

    /// Snodgrass again: `None` while playing; a flag claim closes the
    /// interval just like a mating move does.
    pub fn ended(&self) -> Option<T> {
        self.timeline.ended().or(self.flagged.map(|(_, at)| at))
    }

    /// How much of `color`'s budget is gone as of `now`: the fold of their
    /// think times over the frames, plus the clock running on them right
    /// now if it is their move. Derived, never stored — there is no clock
    /// state to keep wound.
    pub fn spent(&self, color: Color, now: T) -> D {
        let game = self.timeline.game();
        let thought = self
            .timeline
            .frames()
            .enumerate()
            .filter(|&(ply, _)| game[ply].turn() == color)
            .map(|(_, frame)| frame.think_time)
            .fold(D::default(), |total, think_time| total + think_time);
        let latest = self.timeline.latest();
        let ticking =
            self.mode() == Mode::Playing && game.position().turn() == color && now >= latest;
        if ticking {
            thought + (now - latest)
        } else {
            thought
        }
    }

    /// What is left of `color`'s budget as of `now` — the number on the
    /// clock face. Zero means the flag is down and a claim will stick.
    pub fn remaining(&self, color: Color, now: T) -> D
    where
        D: Sub<Output = D>,
    {
        let spent = self.spent(color, now);
        if spent >= self.budget {
            D::default()
        } else {
            self.budget - spent
        }
    }

    /// Pure: returns a new clocked timeline, this one is untouched. A move
    /// stamped after its player's budget ran dry is rejected — it arrived
    /// too late to exist.
    pub fn record(&self, action: impl IntoAction, at: T) -> Result<Clocked<T, D>, Rejected> {
        if let Some((winner, _)) = self.flagged {
            return Err(Rejected::GameOver {
                ending: Ending::Flagged { winner },
            });
        }
        let mover = self.timeline.game().position().turn();
        if at >= self.timeline.latest() && self.spent(mover, at) > self.budget {
            return Err(Rejected::OutOfTime);
        }
        let timeline = self.timeline.record(action, at)?;
        Ok(Clocked {
            timeline,
            budget: self.budget,
            flagged: None,
        })
    }

    /// The forcing mechanism: when the player to move has exceeded their
    /// budget, their opponent may claim the win on time. A claim while the
    /// flag is still up is rejected — time pressure is real, but it is
    /// checked, not assumed.
    pub fn claim_flag(&self, at: T) -> Result<Clocked<T, D>, Rejected> {
        if let Mode::Played(ending) = self.mode() {
            return Err(Rejected::GameOver { ending });
        }
        if at < self.timeline.latest() {
            return Err(Rejected::OutOfOrder);
        }
        let mover = self.timeline.game().position().turn();
        if self.spent(mover, at) > self.budget {
            Ok(Clocked {
                timeline: self.timeline.clone(),
                budget: self.budget,
                flagged: Some((mover.opponent(), at)),
            })
        } else {
            Err(Rejected::StillOnTime)
        }
    }
}
