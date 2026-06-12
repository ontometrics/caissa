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
use crate::piece::{Color, Piece, Role, Wing};
use crate::position::{Position, glyph};
use crate::reduce::{Ending, Mode, Rejected, in_check, legal_actions, mode, reduce};
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

impl San {
    /// The minimal correct description of a legal `action` in `position` —
    /// the inverse of [`San::resolve`]. Disambiguation is computed the way
    /// publications do: nothing if the role-and-target already name one
    /// action, else file, else rank, else the full square.
    pub fn describe(position: Position, action: Action) -> San {
        match action {
            Action::Promote { from, to, into } => San::Promote {
                origin: pawn_origin(from, to),
                to,
                into,
            },
            Action::Move { from, to } => {
                let piece = position.at(from).expect("a legal move starts on a piece");
                if piece.role == Role::King && from.file().abs_diff(to.file()) == 2 {
                    let wing = if to.file() == 6 { Wing::King } else { Wing::Queen };
                    return San::Castle(wing);
                }
                let origin = if piece.role == Role::Pawn {
                    pawn_origin(from, to)
                } else {
                    minimal_origin(position, piece.role, from, to)
                };
                San::Move { role: piece.role, origin, to }
            }
        }
    }
}

/// Pawn convention: captures always name their file, pushes never need to.
fn pawn_origin(from: Square, to: Square) -> Origin {
    if from.file() == to.file() {
        Origin::Anywhere
    } else {
        Origin::File(from.file())
    }
}

fn minimal_origin(position: Position, role: Role, from: Square, to: Square) -> Origin {
    let rivals: Vec<Square> = legal_actions(position)
        .filter_map(|action| match action {
            Action::Move { from: f, to: t }
                if t == to && position.at(f).is_some_and(|piece| piece.role == role) =>
            {
                Some(f)
            }
            _ => None,
        })
        .collect();
    if rivals.len() <= 1 {
        Origin::Anywhere
    } else if rivals.iter().filter(|r| r.file() == from.file()).count() == 1 {
        Origin::File(from.file())
    } else if rivals.iter().filter(|r| r.rank() == from.rank()).count() == 1 {
        Origin::Rank(from.rank())
    } else {
        Origin::Square(from)
    }
}

/// `action` written as SAN — `"Nbd2"`, `"exd6"`, `"O-O-O"`, `"Rd8#"` —
/// suitable for a game score. The check/mate suffix comes from the
/// reducer, so this also validates the action.
pub fn to_san(position: Position, action: Action) -> Result<String, Rejected> {
    notate(position, action, letter)
}

/// `action` in figurine algebraic notation — `"♖d8#"`, `"♛h4#"` — the
/// publication style, with the mover's own glyph.
pub fn to_figurine(position: Position, action: Action) -> Result<String, Rejected> {
    notate(position, action, |role, color| {
        if role == Role::Pawn {
            String::new()
        } else {
            glyph(Piece { color, role }).to_string()
        }
    })
}

fn letter(role: Role, _: Color) -> String {
    match role {
        Role::Pawn => "",
        Role::Knight => "N",
        Role::Bishop => "B",
        Role::Rook => "R",
        Role::Queen => "Q",
        Role::King => "K",
    }
    .to_string()
}

fn notate(
    position: Position,
    action: Action,
    spell: impl Fn(Role, Color) -> String,
) -> Result<String, Rejected> {
    let next = reduce(position, action)?;
    let mover = position.turn();
    let captures = match action {
        Action::Move { from, to } | Action::Promote { from, to, .. } => {
            position.at(to).is_some()
                || (position.at(from).is_some_and(|piece| piece.role == Role::Pawn)
                    && from.file() != to.file())
        }
    };
    let takes = if captures { "x" } else { "" };
    let body = match San::describe(position, action) {
        San::Castle(Wing::King) => "O-O".to_string(),
        San::Castle(Wing::Queen) => "O-O-O".to_string(),
        San::Move { role, origin, to } => {
            format!("{}{}{takes}{to}", spell(role, mover), origin_text(origin))
        }
        San::Promote { origin, to, into } => {
            format!("{}{takes}{to}={}", origin_text(origin), spell(into, mover))
        }
    };
    let suffix = match mode(next) {
        Mode::Played(Ending::Checkmate { .. }) => "#",
        _ if in_check(next, next.turn()) => "+",
        _ => "",
    };
    Ok(format!("{body}{suffix}"))
}

fn origin_text(origin: Origin) -> String {
    match origin {
        Origin::Anywhere => String::new(),
        Origin::File(file) => ((b'a' + file) as char).to_string(),
        Origin::Rank(rank) => ((b'1' + rank) as char).to_string(),
        Origin::Square(square) => square.to_string(),
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
