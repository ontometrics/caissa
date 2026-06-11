use crate::action::{Action, IntoAction};
use crate::piece::{Color, Piece, Role};
use crate::reduce::{Mode, Rejected, in_check, legal_actions, mode, reduce};
use crate::square::Square;

/// A chess position: the board plus whose turn it is.
///
/// `Copy`, so persistence is free — every transition yields a new value and
/// the old one stays valid. History, search trees, and variations are all
/// "keep the old value".
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Position {
    board: [Option<Piece>; 64],
    turn: Color,
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
        Position { board, turn: Color::White }
    }
}

impl Position {
    /// An empty board with the given side to move. Useful for tests and
    /// composed setups via [`Position::with`].
    pub fn empty(turn: Color) -> Position {
        Position { board: [None; 64], turn }
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

    /// Convenience wrapper around [`reduce`] that accepts anything
    /// action-shaped: an [`Action`](crate::Action), a `(Square, Square)`
    /// pair, or a string like `"h2h4"` / `"h7h8q"`.
    pub fn play(self, action: impl IntoAction) -> Result<Position, Rejected> {
        reduce(self, action.into_action()?)
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

    pub(crate) fn moved(self, from: Square, to: Square, piece: Piece) -> Position {
        let mut board = self.board;
        board[from.index()] = None;
        board[to.index()] = Some(piece);
        Position { board, turn: self.turn.opponent() }
    }
}
