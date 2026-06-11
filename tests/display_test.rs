use caissa::Position;
use googletest::prelude::*;

#[test]
fn the_board_renders_as_you_would_draw_it() {
    let board = Position::default().to_string();
    let lines: Vec<&str> = board.lines().collect();

    assert_that!(lines[0], eq("8 ♜ ♞ ♝ ♛ ♚ ♝ ♞ ♜"));
    assert_that!(lines[1], eq("7 ♟ ♟ ♟ ♟ ♟ ♟ ♟ ♟"));
    assert_that!(lines[4], eq("4 · · · · · · · ·"));
    assert_that!(lines[7], eq("1 ♖ ♘ ♗ ♕ ♔ ♗ ♘ ♖"));
    assert_that!(lines[8], eq("  a b c d e f g h"));
}

#[test]
fn a_move_shows_up_on_the_board() {
    let board = Position::default().play("e2e4").unwrap().to_string();
    let lines: Vec<&str> = board.lines().collect();

    assert_that!(lines[4], eq("4 · · · · ♙ · · ·"));
    assert_that!(lines[6], eq("2 ♙ ♙ ♙ ♙ · ♙ ♙ ♙"));
}
