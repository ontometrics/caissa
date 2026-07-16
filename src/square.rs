use std::fmt;
use std::str::FromStr;

use crate::reduce::Rejected;

/// A square on the board: file a–h, rank 1–8.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Square {
    file: u8, // 0 = a-file
    rank: u8, // 0 = rank 1
}

impl Square {
    pub fn new(file: u8, rank: u8) -> Option<Square> {
        (file < 8 && rank < 8).then_some(Square { file, rank })
    }

    /// Const constructor for the [`notation`](crate::notation) constants;
    /// callers guarantee `file < 8 && rank < 8`.
    pub(crate) const fn at(file: u8, rank: u8) -> Square {
        Square { file, rank }
    }

    /// Every square on the board, a1 through h8.
    pub fn all() -> impl Iterator<Item = Square> {
        (0..64u8).map(|index| Square {
            file: index % 8,
            rank: index / 8,
        })
    }

    pub fn file(self) -> u8 {
        self.file
    }

    pub fn rank(self) -> u8 {
        self.rank
    }

    pub(crate) fn index(self) -> usize {
        (self.rank * 8 + self.file) as usize
    }

    pub(crate) fn offset(self, dx: i8, dy: i8) -> Option<Square> {
        let file = self.file as i8 + dx;
        let rank = self.rank as i8 + dy;
        if (0..8).contains(&file) && (0..8).contains(&rank) {
            Some(Square {
                file: file as u8,
                rank: rank as u8,
            })
        } else {
            None
        }
    }
}

impl FromStr for Square {
    type Err = Rejected;

    fn from_str(s: &str) -> Result<Square, Rejected> {
        let &[file, rank] = s.as_bytes() else {
            return Err(Rejected::Unparseable(s.to_string()));
        };
        Square::new(file.wrapping_sub(b'a'), rank.wrapping_sub(b'1'))
            .ok_or_else(|| Rejected::Unparseable(s.to_string()))
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", (b'a' + self.file) as char, self.rank + 1)
    }
}
