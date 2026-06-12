//! SAN — Standard Algebraic Notation, resolved rather than interpreted.
//!
//! "Nf3" is a *description*, not an instruction: parsing yields a [`San`],
//! and resolution filters [`legal_actions`](crate::legal_actions) down to
//! the one action the description admits. SAN therefore inherits every
//! legality rule the reducer enforces — disambiguation included — and
//! there is no second rules engine. Zero matches and several matches are
//! both data: [`Rejected::NoMatch`] and [`Rejected::AmbiguousSan`].

use std::str::FromStr;

use crate::action::Action;
use crate::piece::{Color, Role, Wing};
use crate::position::Position;
use crate::reduce::{Rejected, legal_actions};
use crate::square::Square;

/// What a SAN string asserts about the move it names. The variants mirror
/// [`Action`]'s — `Move` and `Promote` — plus `Castle`, which desugars to
/// the king's two-square move.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum San {
    /// `O-O`, `O-O-O`
    Castle(Wing),
    /// `e4`, `Nf3`, `Nbd2`, `Rxe5`
    Move { role: Role, origin: Origin, to: Square },
    /// `e8=Q`, `exd8=N`
    Promote { origin: Origin, to: Square, into: Role },
}

/// Where the mover comes from — as much as the text cares to say.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Origin {
    /// `Nf3`
    Anywhere,
    /// `Nbd2` — the b-knight
    File(u8),
    /// `R1e2` — the first-rank rook
    Rank(u8),
    /// `Qh4e1` — fully spelled out
    Square(Square),
}

impl Origin {
    fn admits(self, from: Square) -> bool {
        match self {
            Origin::Anywhere => true,
            Origin::File(file) => from.file() == file,
            Origin::Rank(rank) => from.rank() == rank,
            Origin::Square(square) => from == square,
        }
    }
}

impl San {
    /// The unique legal action this notation names in `position`.
    pub fn resolve(self, position: Position) -> Result<Action, Rejected> {
        let candidates: Vec<Action> = match self {
            San::Castle(wing) => {
                // Desugars directly: the king's two-square move for the
                // side to move. Legality is the reducer's business.
                let rank = match position.turn() {
                    Color::White => 0,
                    Color::Black => 7,
                };
                let file = match wing {
                    Wing::King => 6,
                    Wing::Queen => 2,
                };
                return Ok(Action::Move { from: Square::at(4, rank), to: Square::at(file, rank) });
            }
            San::Move { role, origin, to } => legal_actions(position)
                .filter(|action| match *action {
                    Action::Move { from, to: target } => {
                        target == to
                            && origin.admits(from)
                            && position.at(from).is_some_and(|piece| piece.role == role)
                    }
                    Action::Promote { .. } => false,
                })
                .collect(),
            San::Promote { origin, to, into } => legal_actions(position)
                .filter(|action| match *action {
                    Action::Promote { from, to: target, into: role } => {
                        target == to && role == into && origin.admits(from)
                    }
                    Action::Move { .. } => false,
                })
                .collect(),
        };
        match candidates.as_slice() {
            [action] => Ok(*action),
            [] => Err(Rejected::NoMatch { san: self }),
            _ => Err(Rejected::AmbiguousSan { candidates }),
        }
    }
}

fn promotion_role(text: &str) -> Option<Role> {
    match text {
        "Q" => Some(Role::Queen),
        "R" => Some(Role::Rook),
        "B" => Some(Role::Bishop),
        "N" => Some(Role::Knight),
        _ => None,
    }
}

impl FromStr for San {
    type Err = Rejected;

    fn from_str(s: &str) -> Result<San, Rejected> {
        let reject = || Rejected::Unparseable(s.to_string());
        // Check `+`/`#` and annotations are asserted by the writer, not
        // needed to resolve — the reducer knows whether a move checks.
        let bare = s.trim_end_matches(['+', '#', '!', '?']);
        match bare {
            "O-O" | "0-0" => return Ok(San::Castle(Wing::King)),
            "O-O-O" | "0-0-0" => return Ok(San::Castle(Wing::Queen)),
            _ => {}
        }
        let (body, promotion) = match bare.split_once('=') {
            Some((body, suffix)) => (body, Some(promotion_role(suffix).ok_or_else(reject)?)),
            None => (bare, None),
        };
        let mut chars: Vec<char> = body.chars().collect();
        let role = match chars.first() {
            Some('K') => Some(Role::King),
            Some('Q') => Some(Role::Queen),
            Some('R') => Some(Role::Rook),
            Some('B') => Some(Role::Bishop),
            Some('N') => Some(Role::Knight),
            _ => None,
        };
        if role.is_some() {
            chars.remove(0);
        }
        let role = role.unwrap_or(Role::Pawn);
        chars.retain(|&c| c != 'x');
        if chars.len() < 2 {
            return Err(reject());
        }
        let to: Square = chars
            .split_off(chars.len() - 2)
            .into_iter()
            .collect::<String>()
            .parse()
            .map_err(|_| reject())?;
        let origin = match *chars.as_slice() {
            [] => Origin::Anywhere,
            [file @ 'a'..='h'] => Origin::File(file as u8 - b'a'),
            [rank @ '1'..='8'] => Origin::Rank(rank as u8 - b'1'),
            [file @ 'a'..='h', rank @ '1'..='8'] => {
                Origin::Square(Square::at(file as u8 - b'a', rank as u8 - b'1'))
            }
            _ => return Err(reject()),
        };
        match promotion {
            Some(into) if role == Role::Pawn => Ok(San::Promote { origin, to, into }),
            Some(_) => Err(reject()),
            None => Ok(San::Move { role, origin, to }),
        }
    }
}
