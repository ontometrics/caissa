//! The victory lap: classic games folded from their PGN.

use caissa::notation::*;
use caissa::{Color, Ending, Mode, Piece, Rejected, Role, import, pgn};
use googletest::prelude::*;

const OPERA_GAME: &str = r#"
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

const IMMORTAL_GAME: &str = r#"
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

#[test]
fn the_opera_game_folds_to_mate() {
    let game = import(OPERA_GAME).unwrap();

    assert_that!(
        game.mode(),
        eq(Mode::Played(Ending::Checkmate { winner: Color::White }))
    );
    assert_that!(game.plies(), eq(33));
    assert_that!(
        game[Terminus].at(d8),
        some(eq(Piece { color: Color::White, role: Role::Rook }))
    );
}

#[test]
fn the_immortal_game_folds_to_mate() {
    let game = import(IMMORTAL_GAME).unwrap();

    assert_that!(
        game.mode(),
        eq(Mode::Played(Ending::Checkmate { winner: Color::White }))
    );
    assert_that!(game.plies(), eq(45));
    assert_that!(
        game[Terminus].at(e7),
        some(eq(Piece { color: Color::White, role: Role::Bishop }))
    );
}

#[test]
fn tags_come_along() {
    let parsed = pgn::parse(OPERA_GAME).unwrap();

    assert_that!(
        parsed.tags.get("White").map(String::as_str),
        some(eq("Paul Morphy"))
    );
    assert_that!(parsed.result.as_deref(), some(eq("1-0")));
    assert_that!(parsed.sans.len(), eq(33));
}

#[test]
fn comments_move_numbers_and_nags_are_skipped() {
    let game = import("1. e4 {best by test} e5; a classic\n2. Nf3 $1 Nc6 *").unwrap();

    assert_that!(game.plies(), eq(4));
}

#[test]
fn variations_are_rejected_loudly() {
    let result = import("1. e4 (1. d4 d5) e5");

    assert_that!(
        result,
        err(eq(&Rejected::Unparseable(
            "variations (...) are not supported".to_string()
        )))
    );
}

#[test]
fn a_result_that_contradicts_the_board_is_rejected() {
    // Fool's Mate is a win for Black; a PGN claiming 1-0 is lying.
    let result = import("1. f3 e5 2. g4 Qh4# 1-0");

    assert_that!(result, err(anything()));
}
