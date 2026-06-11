use crate::action::Action;
use crate::piece::{Color, Piece, Role, Wing};
use crate::position::Position;
use crate::square::Square;

/// Why an action was not accepted. Errors are data: each variant carries
/// enough to tell the caller exactly what to do instead.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Rejected {
    Unparseable(String),
    EmptySquare { from: Square },
    NotYourTurn { piece: Piece },
    OwnPieceAt { to: Square },
    CannotReach { from: Square, to: Square },
    /// A pawn reached the last rank via `Action::Move`; resend as
    /// `Action::Promote` saying what it becomes.
    NeedsPromotion { from: Square, to: Square },
    /// `Action::Promote` for a move that is not a pawn reaching the last rank.
    NotAPromotion { from: Square, to: Square },
    /// Pawns promote to queen, rook, bishop, or knight — nothing else.
    InvalidPromotion { into: Role },
    /// The move would leave the mover's own king attacked — covers moving
    /// into check, moving a pinned piece, failing to resolve check, and
    /// castling out of or through check.
    IntoCheck { king: Square },
    /// The right to castle on this wing was forfeited — the king or that
    /// rook has moved. Rights never come back.
    CastlingForfeited { wing: Wing },
    /// The game already ended; no action can follow [`Terminus`](crate::Terminus).
    GameOver { ending: Ending },
    /// A [`Timeline`](crate::Timeline) record stamped earlier than the
    /// previous one — time only moves forward.
    OutOfOrder,
    /// A [`Clocked`](crate::Clocked) move stamped after its player's budget
    /// ran dry — the move arrived too late to exist.
    OutOfTime,
    /// A flag claim while the mover still has time on the clock.
    StillOnTime,
}

/// Whether a game is still being played, or has been.
///
/// Deriving it from a position costs a pass over the legal actions
/// ([`mode`]), so it is computed once per accepted move and *carried* —
/// [`Game`](crate::Game) stores its mode and rejects actions on a `Played`
/// game in O(1), with no move checking at all.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    Playing,
    Played(Ending),
}

/// How a played game ended. `Checkmate` and `Stalemate` are derived from
/// the board ([`mode`]): the side to move had no legal action, and check
/// decides which. `Flagged` is produced only by the clock layer
/// ([`Clocked`](crate::Clocked)) — the board cannot see the clock.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Ending {
    Checkmate { winner: Color },
    Stalemate,
    Flagged { winner: Color },
}

/// All actions the side to move can legally take. Pawn moves onto the last
/// rank appear as four `Promote` actions, never as a bare `Move`.
pub fn legal_actions(position: Position) -> impl Iterator<Item = Action> {
    Square::all()
        .filter(move |&from| {
            position
                .at(from)
                .is_some_and(|piece| piece.color == position.turn())
        })
        .flat_map(move |from| Square::all().map(move |to| (from, to)))
        .flat_map(move |(from, to)| candidates(position, from, to))
        .filter(move |&action| reduce(position, action).is_ok())
}

/// Derive a position's [`Mode`]: the side to move either has a legal
/// action, or the game has been played. Costs a pass over the legal
/// actions — carry the result (as [`Game`](crate::Game) does) rather than
/// recomputing it per action.
pub fn mode(position: Position) -> Mode {
    if legal_actions(position).next().is_some() {
        return Mode::Playing;
    }
    if in_check(position, position.turn()) {
        Mode::Played(Ending::Checkmate { winner: position.turn().opponent() })
    } else {
        Mode::Played(Ending::Stalemate)
    }
}

/// Whether `color`'s king is attacked. A board with no king (a composed
/// test position) is never in check.
pub fn in_check(position: Position, color: Color) -> bool {
    threatened_king(position, color).is_some()
}

/// The square of `color`'s king if it is currently attacked.
fn threatened_king(position: Position, color: Color) -> Option<Square> {
    Square::all()
        .find(|&square| position.at(square) == Some(Piece { color, role: Role::King }))
        .filter(|&king| attacked(position, king, color.opponent()))
}

fn attacked(position: Position, target: Square, by: Color) -> bool {
    Square::all().any(|from| {
        position
            .at(from)
            .is_some_and(|piece| piece.color == by && reaches(position, piece, from, target))
    })
}

fn candidates(position: Position, from: Square, to: Square) -> Vec<Action> {
    let last_rank = match position.turn() {
        Color::White => 7,
        Color::Black => 0,
    };
    let promoting = to.rank() == last_rank
        && position
            .at(from)
            .is_some_and(|piece| piece.role == Role::Pawn);
    if promoting {
        [Role::Queen, Role::Rook, Role::Bishop, Role::Knight]
            .into_iter()
            .map(|into| Action::Promote { from, to, into })
            .collect()
    } else {
        vec![Action::Move { from, to }]
    }
}

/// The reducer: a pure transition from one position to the next.
///
/// A game is `actions.try_fold(start, reduce)` — replay, undo, and
/// variations all fall out of that. This checks the *move*, not the game:
/// a position is memoryless, so after checkmate or stalemate each action
/// is still rejected on its merits (the mated side has none). For the
/// explicit [`Rejected::GameOver`] and O(1) gating, go through
/// [`Game`](crate::Game), which carries its [`Mode`]. ([`mode`] is defined
/// in terms of this function, so this must never call [`mode`].)
pub fn reduce(position: Position, action: Action) -> Result<Position, Rejected> {
    let (from, to, promotion) = match action {
        Action::Move { from, to } => (from, to, None),
        Action::Promote { from, to, into } => (from, to, Some(into)),
    };

    let piece = position.at(from).ok_or(Rejected::EmptySquare { from })?;
    if piece.color != position.turn() {
        return Err(Rejected::NotYourTurn { piece });
    }
    if from == to {
        return Err(Rejected::CannotReach { from, to });
    }
    // Castling needs no notation of its own: it is the king's two-square
    // move (e1g1 / e1c1, UCI-style). A king can never legally travel two
    // squares any other way, so intent stays from–to.
    if piece.role == Role::King
        && from == king_home(piece.color)
        && let Some(wing) = castle_wing(piece.color, to)
    {
        return castle(position, piece.color, wing);
    }
    if position.at(to).is_some_and(|target| target.color == piece.color) {
        return Err(Rejected::OwnPieceAt { to });
    }
    if !reaches(position, piece, from, to) {
        return Err(Rejected::CannotReach { from, to });
    }

    let last_rank = match piece.color {
        Color::White => 7,
        Color::Black => 0,
    };
    let promotes = piece.role == Role::Pawn && to.rank() == last_rank;
    let role = match (promotion, promotes) {
        (None, false) => piece.role,
        (None, true) => return Err(Rejected::NeedsPromotion { from, to }),
        (Some(_), false) => return Err(Rejected::NotAPromotion { from, to }),
        (Some(into @ (Role::Pawn | Role::King)), true) => {
            return Err(Rejected::InvalidPromotion { into });
        }
        (Some(into), true) => into,
    };

    let next = position.moved(from, to, Piece { color: piece.color, role });
    if let Some(king) = threatened_king(next, piece.color) {
        return Err(Rejected::IntoCheck { king });
    }
    Ok(next)
}

fn reaches(position: Position, piece: Piece, from: Square, to: Square) -> bool {
    let dx = to.file() as i8 - from.file() as i8;
    let dy = to.rank() as i8 - from.rank() as i8;
    match piece.role {
        Role::Pawn => pawn_reaches(position, piece.color, from, to, dx, dy),
        Role::Knight => (dx.abs() == 1 && dy.abs() == 2) || (dx.abs() == 2 && dy.abs() == 1),
        Role::Bishop => dx.abs() == dy.abs() && path_clear(position, from, to),
        Role::Rook => (dx == 0 || dy == 0) && path_clear(position, from, to),
        Role::Queen => {
            (dx.abs() == dy.abs() || dx == 0 || dy == 0) && path_clear(position, from, to)
        }
        Role::King => dx.abs().max(dy.abs()) == 1,
    }
}

fn king_home(color: Color) -> Square {
    match color {
        Color::White => Square::at(4, 0),
        Color::Black => Square::at(4, 7),
    }
}

fn castle_wing(color: Color, to: Square) -> Option<Wing> {
    let rank = match color {
        Color::White => 0,
        Color::Black => 7,
    };
    if to == Square::at(6, rank) {
        Some(Wing::King)
    } else if to == Square::at(2, rank) {
        Some(Wing::Queen)
    } else {
        None
    }
}

fn castle(position: Position, color: Color, wing: Wing) -> Result<Position, Rejected> {
    let rank = match color {
        Color::White => 0,
        Color::Black => 7,
    };
    let at = |file| Square::at(file, rank);
    let king_from = at(4);
    let (king_to, rook_from, rook_to) = match wing {
        Wing::King => (at(6), at(7), at(5)),
        Wing::Queen => (at(2), at(0), at(3)),
    };
    if !position.may_castle(color, wing) {
        return Err(Rejected::CastlingForfeited { wing });
    }
    let kingside_between = [at(5), at(6)];
    let queenside_between = [at(1), at(2), at(3)];
    let between: &[Square] = match wing {
        Wing::King => &kingside_between,
        Wing::Queen => &queenside_between,
    };
    let rook_present = position.at(rook_from) == Some(Piece { color, role: Role::Rook });
    if !rook_present || between.iter().any(|&square| position.at(square).is_some()) {
        return Err(Rejected::CannotReach { from: king_from, to: king_to });
    }
    // The king may not castle out of, through, or into an attacked square.
    // The square he passes through is exactly where the rook lands, on
    // both wings.
    let kings_path = [king_from, rook_to, king_to];
    for square in kings_path {
        if attacked(position, square, color.opponent()) {
            return Err(Rejected::IntoCheck { king: square });
        }
    }
    Ok(position.castled(color, (king_from, king_to), (rook_from, rook_to)))
}

fn pawn_reaches(position: Position, color: Color, from: Square, to: Square, dx: i8, dy: i8) -> bool {
    let dir: i8 = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let start_rank = match color {
        Color::White => 1,
        Color::Black => 6,
    };
    let en_passant = Some(to) == position.passant();
    let push = dx == 0 && dy == dir && position.at(to).is_none();
    let double = dx == 0
        && dy == 2 * dir
        && from.rank() == start_rank
        && from
            .offset(0, dir)
            .is_some_and(|mid| position.at(mid).is_none())
        && position.at(to).is_none();
    let capture = dx.abs() == 1 && dy == dir && (position.at(to).is_some() || en_passant);
    push || double || capture
}

fn path_clear(position: Position, from: Square, to: Square) -> bool {
    let dx = (to.file() as i8 - from.file() as i8).signum();
    let dy = (to.rank() as i8 - from.rank() as i8).signum();
    let mut step = from.offset(dx, dy);
    while let Some(square) = step {
        if square == to {
            return true;
        }
        if position.at(square).is_some() {
            return false;
        }
        step = square.offset(dx, dy);
    }
    false
}
