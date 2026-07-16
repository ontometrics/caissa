//! Reading games: the annotator.
//!
//! A game-level interpreter, one altitude above the move-interpreter.
//! `expand`/`apply` turn a move into board edits — what happened on the
//! squares. [`annotate`] turns a *game* into a narrative of decisions —
//! what happened in the contest. It is the shift series: fold the game's
//! history through a position evaluator and difference it, so each move
//! carries how much it swung the position from its *mover's* point of
//! view. A sharply negative swing is a fumble; the worst is the turning
//! point.
//!
//! The verdict is only as honest as the evaluator — it is
//! estimator-relative. [`material`](crate::play::material) alone catches
//! material swings; a search evaluator
//! ([`evaluate`](crate::play::evaluate)) catches tactics and mates. Clear
//! fumbles — a hung queen, a missed mate — are reliable; subtle ones need
//! a stronger judge.
//!
//! ```
//! use caissa::classics::fools_mate;
//! use caissa::play::{evaluate, material};
//! use caissa::review::annotate;
//!
//! // a one-ply tactical eval, enough to see the mate
//! let series = annotate(&fools_mate(), &|p| evaluate(p, 1, &material));
//! let turning_point = series.iter().min_by_key(|a| a.swing).unwrap();
//! // turning_point.ply == 3 — White's 2. g4?? allowed Qh4#
//! # assert!(turning_point.ply == 3);
//! ```

use crate::action::Action;
use crate::game::Game;
use crate::piece::Color;
use crate::position::Position;

/// What one move did to the position, from the mover's point of view.
/// `swing` is the change in the position's value across the move (in the
/// evaluator's units); sharply negative is a fumble.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Assessment {
    pub ply: usize,
    pub mover: Color,
    pub action: Action,
    pub swing: i32,
}

/// The shift series: every move scored by how much it swung the position
/// for the player who made it. A fold over the game's history through
/// `eval`, then a difference — `min_by_key(|a| a.swing)` is the turning
/// point, `filter(|a| a.swing < -t)` the fumbles.
pub fn annotate(game: &Game, eval: &impl Fn(Position) -> i32) -> Vec<Assessment> {
    (0..game.plies())
        .map(|ply| {
            let mover = game[ply].turn();
            let before = white_pov(game[ply], eval);
            let after = white_pov(game[ply + 1], eval);
            let delta = after - before; // from White's view
            let swing = match mover {
                Color::White => delta,
                Color::Black => -delta,
            };
            Assessment {
                ply: ply + 1,
                mover,
                action: game.log()[ply],
                swing,
            }
        })
        .collect()
}

/// An evaluator reports from the side to move's view; normalize to White's
/// so successive plies are comparable across the alternating turn.
fn white_pov(position: Position, eval: &impl Fn(Position) -> i32) -> i32 {
    match position.turn() {
        Color::White => eval(position),
        Color::Black => -eval(position),
    }
}
