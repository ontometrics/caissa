#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn opponent(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

/// Which side of the board a castle happens on.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Wing {
    King,
    Queen,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Role {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Piece {
    pub color: Color,
    pub role: Role,
}

impl Piece {
    pub fn white(role: Role) -> Piece {
        Piece {
            color: Color::White,
            role,
        }
    }

    pub fn black(role: Role) -> Piece {
        Piece {
            color: Color::Black,
            role,
        }
    }
}
