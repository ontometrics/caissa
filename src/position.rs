use std::fmt;

use crate::action::{Action, IntoAction};
use crate::piece::{Color, Piece, Role, Wing};
use crate::reduce::{Edit, Mode, Rejected, in_check, legal_actions, mode, reduce};
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
/// forfeits both wings, touching a corner square forfeits that wing — and
/// the law lives in the API itself: `without` exists, `with` does not.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct Rights {
    remaining: [bool; 4],
}

const CORNERS: [(Square, Color, Wing); 4] = [
    (Square::at(7, 0), Color::White, Wing::King),
    (Square::at(0, 0), Color::White, Wing::Queen),
    (Square::at(7, 7), Color::Black, Wing::King),
    (Square::at(0, 7), Color::Black, Wing::Queen),
];

impl Rights {
    const ALL: Rights = Rights { remaining: [true; 4] };

    fn slot(color: Color, wing: Wing) -> usize {
        match (color, wing) {
            (Color::White, Wing::King) => 0,
            (Color::White, Wing::Queen) => 1,
            (Color::Black, Wing::King) => 2,
            (Color::Black, Wing::Queen) => 3,
        }
    }

    fn allows(self, color: Color, wing: Wing) -> bool {
        self.remaining[Rights::slot(color, wing)]
    }

    fn without(self, color: Color, wing: Wing) -> Rights {
        let mut remaining = self.remaining;
        remaining[Rights::slot(color, wing)] = false;
        Rights { remaining }
    }
}

/// The standard starting position — what `Game::new()` begins from: an
/// empty board with both armies placed, White to move, full castling
/// rights. (For sparse test boards, start from [`Position::empty`] and
/// compose with [`Position::with`] instead.)
impl Default for Position {
    fn default() -> Position {
        let back_rank = [
            Role::Rook,
            Role::Knight,
            Role::Bishop,
            Role::Queen,
            Role::King,
            Role::Bishop,
            Role::Knight,
            Role::Rook,
        ];
        let mut position = Position::empty(Color::White);
        for (file, role) in back_rank.into_iter().enumerate() {
            let file = file as u8;
            position = position
                .with(Square::at(file, 0), Piece::white(role)) // rank 1
                .with(Square::at(file, 1), Piece::white(Role::Pawn)) // rank 2
                .with(Square::at(file, 6), Piece::black(Role::Pawn)) // rank 7
                .with(Square::at(file, 7), Piece::black(role)); // rank 8
        }
        position
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

    /// This position as FEN. A bare position cannot know the two
    /// counters (they are history, not state), so they default to
    /// `0 1` — [`Game::fen`](crate::Game::fen) supplies the real ones.
    pub fn fen(self) -> String {
        self.fen_with(0, 1)
    }

    pub(crate) fn fen_with(self, halfmove: usize, fullmove: usize) -> String {
        let mut out = String::new();
        for rank in (0..8).rev() {
            let mut run = 0;
            for file in 0..8 {
                match self.board[(rank * 8 + file) as usize] {
                    Some(piece) => {
                        if run > 0 {
                            out.push_str(&run.to_string());
                            run = 0;
                        }
                        out.push(fen_letter(piece));
                    }
                    None => run += 1,
                }
            }
            if run > 0 {
                out.push_str(&run.to_string());
            }
            if rank > 0 {
                out.push('/');
            }
        }
        out.push(' ');
        out.push(match self.turn {
            Color::White => 'w',
            Color::Black => 'b',
        });
        out.push(' ');
        let castling: String = [
            (Color::White, Wing::King, 'K'),
            (Color::White, Wing::Queen, 'Q'),
            (Color::Black, Wing::King, 'k'),
            (Color::Black, Wing::Queen, 'q'),
        ]
        .into_iter()
        .filter(|&(color, wing, _)| self.rights.allows(color, wing))
        .map(|(_, _, letter)| letter)
        .collect();
        out.push_str(if castling.is_empty() { "-" } else { &castling });
        out.push(' ');
        match self.passant {
            Some(square) => out.push_str(&square.to_string()),
            None => out.push('-'),
        }
        out.push_str(&format!(" {halfmove} {fullmove}"));
        out
    }

    /// A position read from FEN. The two counters are accepted but not
    /// kept — they are history, and a position is state; a game built
    /// from an imported position counts quiet plies from the import.
    pub fn from_fen(fen: &str) -> Result<Position, Rejected> {
        let reject = || Rejected::Unparseable(fen.to_string());
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() < 4 {
            return Err(reject());
        }

        let mut board = [None; 64];
        let ranks: Vec<&str> = fields[0].split('/').collect();
        if ranks.len() != 8 {
            return Err(reject());
        }
        for (row, text) in ranks.iter().enumerate() {
            let rank = 7 - row as u8;
            let mut file = 0u8;
            for c in text.chars() {
                if let Some(run) = c.to_digit(10) {
                    file += run as u8;
                } else {
                    if file >= 8 {
                        return Err(reject());
                    }
                    board[(rank * 8 + file) as usize] = Some(fen_piece(c).ok_or_else(reject)?);
                    file += 1;
                }
            }
            if file != 8 {
                return Err(reject());
            }
        }

        let turn = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(reject()),
        };

        let mut remaining = [false; 4];
        if fields[2] != "-" {
            for c in fields[2].chars() {
                let (color, wing) = match c {
                    'K' => (Color::White, Wing::King),
                    'Q' => (Color::White, Wing::Queen),
                    'k' => (Color::Black, Wing::King),
                    'q' => (Color::Black, Wing::Queen),
                    _ => return Err(reject()),
                };
                remaining[Rights::slot(color, wing)] = true;
            }
        }

        let passant = match fields[3] {
            "-" => None,
            square => Some(square.parse().map_err(|_| reject())?),
        };

        Ok(Position { board, turn, rights: Rights { remaining }, passant })
    }

    /// The interpreter's back half: a total evaluator that folds a move's
    /// [`Edit`]s over the board. The en-passant window resets every move,
    /// reopened only when a `Skip` edit says a pawn just ran past. Rights
    /// are bookkept per `Lift` — lifting a king forfeits both its wings,
    /// lifting (or vacating, or capturing on) a corner forfeits that wing
    /// — so castling, rook moves, and rook captures all pay the same way
    /// without anyone special-casing them.
    pub(crate) fn apply(self, edits: &[Edit]) -> Position {
        let mut board = self.board;
        let mut rights = self.rights;
        let mut passant = None;
        for edit in edits {
            match *edit {
                Edit::Lift(square) => {
                    if let Some(Piece { color, role: Role::King }) = board[square.index()] {
                        rights = rights.without(color, Wing::King).without(color, Wing::Queen);
                    }
                    for (corner, color, wing) in CORNERS {
                        if square == corner {
                            rights = rights.without(color, wing);
                        }
                    }
                    board[square.index()] = None;
                }
                Edit::Place(square, piece) => board[square.index()] = Some(piece),
                Edit::Skip(square) => passant = Some(square),
            }
        }
        Position { board, turn: self.turn.opponent(), rights, passant }
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

pub(crate) fn fen_letter(piece: Piece) -> char {
    let letter = match piece.role {
        Role::Pawn => 'p',
        Role::Knight => 'n',
        Role::Bishop => 'b',
        Role::Rook => 'r',
        Role::Queen => 'q',
        Role::King => 'k',
    };
    match piece.color {
        Color::White => letter.to_ascii_uppercase(),
        Color::Black => letter,
    }
}

fn fen_piece(letter: char) -> Option<Piece> {
    let role = match letter.to_ascii_lowercase() {
        'p' => Role::Pawn,
        'n' => Role::Knight,
        'b' => Role::Bishop,
        'r' => Role::Rook,
        'q' => Role::Queen,
        'k' => Role::King,
        _ => return None,
    };
    let color = if letter.is_ascii_uppercase() { Color::White } else { Color::Black };
    Some(Piece { color, role })
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
