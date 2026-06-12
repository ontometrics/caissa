//! The classics: famous games and standard openings as ready-made values.
//!
//! These are fixtures in the broadest sense — for tests, doc examples,
//! demos, and chapters. A chess library should ship its own literature:
//! when you need "a finished game", "a position where castling is legal",
//! or "the most famous attack ever played", they are one call away.

use crate::game::Game;
use crate::pgn::import;
use crate::position::Position;

/// Morphy vs. Duke Karl / Count Isouard, Paris Opera 1858. Mate in 17 —
/// the most-taught game in chess.
pub const OPERA_GAME_PGN: &str = r#"
[Event "Paris Opera"]
[Site "Paris FRA"]
[Date "1858.11.02"]
[White "Paul Morphy"]
[Black "Duke Karl / Count Isouard"]
[Result "1-0"]

1. e4 e5 2. Nf3 d6 3. d4 Bg4 4. dxe5 Bxf3 5. Qxf3 dxe5 6. Bc4 Nf6
7. Qb3 Qe7 8. Nc3 c6 9. Bg5 b5 10. Nxb5 cxb5 11. Bxb5+ Nbd7 12. O-O-O Rd8
13. Rxd7 Rxd7 14. Rd1 Qe6 15. Bxd7+ Nxd7 16. Qb8+ Nxb8 17. Rd8# 1-0
"#;

/// Anderssen vs. Kieseritzky, London 1851 — both rooks and the queen
/// sacrificed, mate with three minor pieces.
pub const IMMORTAL_GAME_PGN: &str = r#"
[Event "London"]
[Site "London ENG"]
[Date "1851.06.21"]
[White "Adolf Anderssen"]
[Black "Lionel Kieseritzky"]
[Result "1-0"]

1. e4 e5 2. f4 exf4 3. Bc4 Qh4+ 4. Kf1 b5 5. Bxb5 Nf6 6. Nf3 Qh6
7. d3 Nh5 8. Nh4 Qg5 9. Nf5 c6 10. g4 Nf6 11. Rg1 cxb5 12. h4 Qg6
13. h5 Qg5 14. Qf3 Ng8 15. Bxf4 Qf6 16. Nc3 Bc5 17. Nd5 Qxb2 18. Bd6 Bxg1
19. e5 Qxa1+ 20. Ke2 Na6 21. Nxg7+ Kd8 22. Qf6+ Nxf6 23. Be7# 1-0
"#;

/// The Opera Game, already played.
pub fn opera_game() -> Game {
    import(OPERA_GAME_PGN).expect("the classics are legal")
}

/// The Immortal Game, already played.
pub fn immortal_game() -> Game {
    import(IMMORTAL_GAME_PGN).expect("the classics are legal")
}

/// The fastest possible mate: 1. f3 e5 2. g4 Qh4#. Black wins; White
/// regrets. The canonical finished-game fixture.
pub fn fools_mate() -> Game {
    ["f3", "e5", "g4", "Qh4#"]
        .into_iter()
        .try_fold(Game::new(), |game, san| game.apply(san))
        .expect("the classics are legal")
}

/// The Ruy Lopez after 3. Bb5 — five plies of the oldest opening in the
/// book, Black to move.
pub fn ruy_lopez() -> Game {
    ["e4", "e5", "Nf3", "Nc6", "Bb5"]
        .into_iter()
        .try_fold(Game::new(), |game, san| game.apply(san))
        .expect("the classics are legal")
}

/// The Italian Game after 3... Bc5 — both kingsides clear, castling
/// legal for either side. The canonical may-castle fixture.
pub fn italian() -> Position {
    ["e4", "e5", "Nf3", "Nf6", "Bc4", "Bc5"]
        .into_iter()
        .try_fold(Position::default(), Position::play)
        .expect("the classics are legal")
}
