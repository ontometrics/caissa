//! A functional chess library.
//!
//! Positions are plain `Copy` values, moves are pure transitions, and a game
//! is a fold of actions over a starting position. An [`Action`] carries no
//! more than the player's intent — a from-square and a to-square — because
//! the position already knows everything else (what stands on the square,
//! whether the target is a capture). The one exception chess forces is
//! promotion, which gets its own action variant.
//!
//! ```
//! use caissa::Position;
//!
//! let position = ["e2e4", "e7e5", "g1f3"]
//!     .into_iter()
//!     .try_fold(Position::default(), Position::play)?;
//! # Ok::<(), caissa::Rejected>(())
//! ```
//!
//! Or in operator notation (`->` is not overloadable in Rust; `>>` is the
//! arrow-shaped operator that is):
//!
//! ```
//! use caissa::Position;
//! use caissa::notation::*;
//!
//! let position = (Position::default() + (e2 >> e4) + (e7 >> e5) + (g1 >> f3))?;
//! # Ok::<(), caissa::Rejected>(())
//! ```
//!
//! The reducer enforces full legality around the king: a move that leaves
//! your own king attacked is rejected ([`Rejected::IntoCheck`] — one rule
//! that covers pins, moving into check, and ignoring check). A game is
//! either being played or has been played ([`Mode`]); deriving that from a
//! position costs a pass over the legal actions, so [`Game`] computes it
//! once per accepted move and carries it — once `Played`, no move checking
//! is done at all, and every action is [`Rejected::GameOver`] in O(1).
//! There is no position after the end of a game.
//!
//! Castling is the king's two-square move (`e1g1` / `e1c1`, UCI-style) —
//! it needs no notation of its own, and `O-O` is import-time sugar. En
//! passant is likewise just the diagonal pawn move onto the skipped
//! square. Both cost [`Position`] its first memory: castling rights and
//! the en-passant square ride along in the value, exactly the fields FEN
//! has always carried.
//!
//! Not yet implemented:
//! - draw rules (repetition, fifty-move, insufficient material)

mod action;
mod clock;
mod game;
pub mod notation;
mod ops;
mod piece;
mod position;
mod reduce;
mod square;
mod timeline;

pub use action::{Action, IntoAction};
pub use clock::Clocked;
pub use game::{Game, Ply, Terminus};
pub use piece::{Color, Piece, Role, Wing};
pub use position::Position;
pub use reduce::{Ending, Mode, Rejected, in_check, legal_actions, mode, reduce};
pub use square::Square;
pub use timeline::{Frame, Timeline};
