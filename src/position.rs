use std::fmt;

use crate::action::{Action, IntoAction};
use crate::piece::{Color, Piece, Role, Wing};
use crate::reduce::{Change, Edit, Mode, Rejected, in_check, legal_actions, mode, reduce};
use crate::square::Square;

/// A chess position: the board, whose turn it is, and the two scraps of
/// memory the rules of chess force a position to carry — castling rights
/// and the en-passant square. Everything else about the past is genuinely
/// forgotten.
///
/// `Copy`, so persistence is free — every transition yields a new value and
/// the old one stays valid. History, search trees, and variations are all
/// "keep the old value".
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Position {
    board: [Option<Piece>; 64],
    turn: Color,
    rights: Rights,
    passant: Option<Square>,
}

/// Which castles remain available. Rights only ever shrink: moving a king
/// clears both wings, touching a corner square clears that wing, and
/// nothing restores them.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct Rights([bool; 4]);

const CORNERS: [(Square, Color, Wing); 4] = [
    (Square::at(7, 0), Color::White, Wing::King),
    (Square::at(0, 0), Color::White, Wing::Queen),
    (Square::at(7, 7), Color::Black, Wing::King),
    (Square::at(0, 7), Color::Black, Wing::Queen),
];

impl Rights {
    const ALL: Rights = Rights([true; 4]);

    fn index(color: Color, wing: Wing) -> usize {
        match (color, wing) {
            (Color::White, Wing::King) => 0,
            (Color::White, Wing::Queen) => 1,
            (Color::Black, Wing::King) => 2,
            (Color::Black, Wing::Queen) => 3,
        }
    }

    fn allows(self, color: Color, wing: Wing) -> bool {
        self.0[Rights::index(color, wing)]
    }

    fn clear(&mut self, color: Color, wing: Wing) {
        self.0[Rights::index(color, wing)] = false;
    }
}

impl Default for Position {
    fn default() -> Position {
        let mut board = [None; 64];
        let back = [
            Role::Rook,
            Role::Knight,
            Role::Bishop,
            Role::Queen,
            Role::King,
            Role::Bishop,
            Role::Knight,
            Role::Rook,
        ];
        for (file, role) in back.into_iter().enumerate() {
            board[file] = Some(Piece { color: Color::White, role });
            board[8 + file] = Some(Piece { color: Color::White, role: Role::Pawn });
            board[48 + file] = Some(Piece { color: Color::Black, role: Role::Pawn });
            board[56 + file] = Some(Piece { color: Color::Black, role });
        }
        Position { board, turn: Color::White, rights: Rights::ALL, passant: None }
    }
}

impl Position {
    /// An empty board with the given side to move. Useful for tests and
    /// composed setups via [`Position::with`]. Composed positions keep full
    /// castling rights — castling legality still requires the king and
    /// rook to actually stand on their home squares.
    pub fn empty(turn: Color) -> Position {
        Position { board: [None; 64], turn, rights: Rights::ALL, passant: None }
    }

    /// A new position with `piece` placed on `square`.
    pub fn with(self, square: Square, piece: Piece) -> Position {
        let mut board = self.board;
        board[square.index()] = Some(piece);
        Position { board, ..self }
    }

    pub fn at(self, square: Square) -> Option<Piece> {
        self.board[square.index()]
    }

    pub fn turn(self) -> Color {
        self.turn
    }

    /// The square a just-double-pushed pawn skipped, capturable en passant
    /// this move only.
    pub fn passant(self) -> Option<Square> {
        self.passant
    }

    /// Whether `color` still has the right to castle on `wing` — the king
    /// and that rook have never moved.
    pub fn may_castle(self, color: Color, wing: Wing) -> bool {
        self.rights.allows(color, wing)
    }

    /// Convenience wrapper around [`reduce`] that accepts anything
    /// action-shaped: an [`Action`](crate::Action), a `(Square, Square)`
    /// pair, or a string like `"h2h4"` / `"h7h8q"`.
    pub fn play(self, action: impl IntoAction) -> Result<Position, Rejected> {
        reduce(self, action.into_action(self)?)
    }

    /// Derive this position's [`Mode`] — playing, or played. Costs a pass
    /// over the legal actions; [`Game`](crate::Game) memoizes it instead.
    pub fn mode(self) -> Mode {
        mode(self)
    }

    /// All actions the side to move can legally take.
    pub fn legal_actions(self) -> impl Iterator<Item = Action> {
        legal_actions(self)
    }

    /// The move the position forces, if the side to move has exactly one.
    pub fn forced(self) -> Option<Action> {
        let mut actions = legal_actions(self);
        let only = actions.next()?;
        actions.next().is_none().then_some(only)
    }

    /// Whether `color`'s king is attacked.
    pub fn in_check(self, color: Color) -> bool {
        in_check(self, color)
    }

    /// FIDE's "same position" for repetition (rule 9.2): board, turn, and
    /// castling rights — but the en-passant square only when an enemy pawn
    /// actually stands ready to take it. This is a *domain equivalence*,
    /// coarser than `Eq`: two positions can differ structurally (one
    /// records a skipped square no pawn can use) yet repeat by the rules.
    /// Compare keys, not positions, when counting repetitions.
    pub fn repetition_key(self) -> Position {
        let live = self.passant.filter(|&skipped| self.ep_capturable(skipped));
        Position { passant: live, ..self }
    }

    fn ep_capturable(self, skipped: Square) -> bool {
        let pusher: i8 = match self.turn.opponent() {
            Color::White => 1,
            Color::Black => -1,
        };
        [-1i8, 1].into_iter().any(|dx| {
            skipped.offset(dx, pusher).is_some_and(|square| {
                self.at(square) == Some(Piece { color: self.turn, role: Role::Pawn })
            })
        })
    }

    /// The interpreter's back half: a total evaluator that folds a
    /// [`Change`]'s edits over the board. Rights are bookkept per edit —
    /// lifting a king forfeits both its wings, lifting (or vacating, or
    /// capturing on) a corner forfeits that wing — so castling, rook
    /// moves, and rook captures all pay the same way without anyone
    /// special-casing them.
    pub(crate) fn apply(self, change: &Change) -> Position {
        let mut board = self.board;
        let mut rights = self.rights;
        for edit in &change.edits {
            match *edit {
                Edit::Lift(square) => {
                    if let Some(Piece { color, role: Role::King }) = board[square.index()] {
                        rights.clear(color, Wing::King);
                        rights.clear(color, Wing::Queen);
                    }
                    for (corner, color, wing) in CORNERS {
                        if square == corner {
                            rights.clear(color, wing);
                        }
                    }
                    board[square.index()] = None;
                }
                Edit::Place(square, piece) => board[square.index()] = Some(piece),
            }
        }
        Position { board, turn: self.turn.opponent(), rights, passant: change.passant }
    }
}

/// The board as you'd draw it: rank 8 at the top, files lettered along the
/// bottom, Unicode pieces, `·` for empty squares.
impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            write!(f, "{} ", rank + 1)?;
            for file in 0..8 {
                let glyph = match self.board[(rank * 8 + file) as usize] {
                    Some(piece) => glyph(piece),
                    None => '·',
                };
                write!(f, "{glyph}")?;
                if file < 7 {
                    write!(f, " ")?;
                }
            }
            writeln!(f)?;
        }
        write!(f, "  a b c d e f g h")
    }
}

pub(crate) fn glyph(piece: Piece) -> char {
    match (piece.color, piece.role) {
        (Color::White, Role::King) => '♔',
        (Color::White, Role::Queen) => '♕',
        (Color::White, Role::Rook) => '♖',
        (Color::White, Role::Bishop) => '♗',
        (Color::White, Role::Knight) => '♘',
        (Color::White, Role::Pawn) => '♙',
        (Color::Black, Role::King) => '♚',
        (Color::Black, Role::Queen) => '♛',
        (Color::Black, Role::Rook) => '♜',
        (Color::Black, Role::Bishop) => '♝',
        (Color::Black, Role::Knight) => '♞',
        (Color::Black, Role::Pawn) => '♟',
    }
}
