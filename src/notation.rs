//! Algebraic-notation constants: `e4`, `g1`, … plus lowercase role names,
//! so actions read like chess: `e2 >> e4`, `e7 >> e8 >> queen`.
#![allow(non_upper_case_globals)]

use crate::piece::Role;
use crate::square::Square;

pub use crate::game::Terminus;

pub const a1: Square = Square::at(0, 0);
pub const b1: Square = Square::at(1, 0);
pub const c1: Square = Square::at(2, 0);
pub const d1: Square = Square::at(3, 0);
pub const e1: Square = Square::at(4, 0);
pub const f1: Square = Square::at(5, 0);
pub const g1: Square = Square::at(6, 0);
pub const h1: Square = Square::at(7, 0);

pub const a2: Square = Square::at(0, 1);
pub const b2: Square = Square::at(1, 1);
pub const c2: Square = Square::at(2, 1);
pub const d2: Square = Square::at(3, 1);
pub const e2: Square = Square::at(4, 1);
pub const f2: Square = Square::at(5, 1);
pub const g2: Square = Square::at(6, 1);
pub const h2: Square = Square::at(7, 1);

pub const a3: Square = Square::at(0, 2);
pub const b3: Square = Square::at(1, 2);
pub const c3: Square = Square::at(2, 2);
pub const d3: Square = Square::at(3, 2);
pub const e3: Square = Square::at(4, 2);
pub const f3: Square = Square::at(5, 2);
pub const g3: Square = Square::at(6, 2);
pub const h3: Square = Square::at(7, 2);

pub const a4: Square = Square::at(0, 3);
pub const b4: Square = Square::at(1, 3);
pub const c4: Square = Square::at(2, 3);
pub const d4: Square = Square::at(3, 3);
pub const e4: Square = Square::at(4, 3);
pub const f4: Square = Square::at(5, 3);
pub const g4: Square = Square::at(6, 3);
pub const h4: Square = Square::at(7, 3);

pub const a5: Square = Square::at(0, 4);
pub const b5: Square = Square::at(1, 4);
pub const c5: Square = Square::at(2, 4);
pub const d5: Square = Square::at(3, 4);
pub const e5: Square = Square::at(4, 4);
pub const f5: Square = Square::at(5, 4);
pub const g5: Square = Square::at(6, 4);
pub const h5: Square = Square::at(7, 4);

pub const a6: Square = Square::at(0, 5);
pub const b6: Square = Square::at(1, 5);
pub const c6: Square = Square::at(2, 5);
pub const d6: Square = Square::at(3, 5);
pub const e6: Square = Square::at(4, 5);
pub const f6: Square = Square::at(5, 5);
pub const g6: Square = Square::at(6, 5);
pub const h6: Square = Square::at(7, 5);

pub const a7: Square = Square::at(0, 6);
pub const b7: Square = Square::at(1, 6);
pub const c7: Square = Square::at(2, 6);
pub const d7: Square = Square::at(3, 6);
pub const e7: Square = Square::at(4, 6);
pub const f7: Square = Square::at(5, 6);
pub const g7: Square = Square::at(6, 6);
pub const h7: Square = Square::at(7, 6);

pub const a8: Square = Square::at(0, 7);
pub const b8: Square = Square::at(1, 7);
pub const c8: Square = Square::at(2, 7);
pub const d8: Square = Square::at(3, 7);
pub const e8: Square = Square::at(4, 7);
pub const f8: Square = Square::at(5, 7);
pub const g8: Square = Square::at(6, 7);
pub const h8: Square = Square::at(7, 7);

pub const queen: Role = Role::Queen;
pub const rook: Role = Role::Rook;
pub const bishop: Role = Role::Bishop;
pub const knight: Role = Role::Knight;
