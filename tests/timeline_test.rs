use caissa::notation::*;
use caissa::{Color, Game, Piece, Position, Role};
use googletest::prelude::*;

fn fools_mate() -> Game {
    ["f2f3", "e7e5", "g2g4", "d8h4"]
        .into_iter()
        .try_fold(Game::new(), |game, action| game.apply(action))
        .unwrap()
}

fn ruy_lopez_start() -> Game {
    ["e2e4", "e7e5", "g1f3", "b8c6", "f1b5"]
        .into_iter()
        .try_fold(Game::new(), |game, action| game.apply(action))
        .unwrap()
}

#[test]
fn game_zero_is_the_start() {
    let game = ruy_lopez_start();

    assert_that!(game[0], eq(Position::default()));
}

#[test]
fn jump_notation_reaches_any_ply() {
    let game = ruy_lopez_start();

    assert_that!(
        game[1].at(e4),
        some(eq(Piece { color: Color::White, role: Role::Pawn }))
    );
    assert_that!(game[2].at(e5), some(eq(Piece { color: Color::Black, role: Role::Pawn })));
    assert_that!(game[game.plies()], eq(game.position()));
}

#[test]
fn white_makes_the_odd_plies() {
    let game = ruy_lopez_start();

    for ply in 1..=game.plies() {
        let mover = game[ply - 1].turn();
        let expected = if ply % 2 == 1 { Color::White } else { Color::Black };
        assert_that!(mover, eq(expected));
    }
}

#[test]
fn position_at_is_the_checked_form() {
    let game = ruy_lopez_start();

    assert_that!(game.position_at(5), some(eq(game.position())));
    assert_that!(game.position_at(6), none());
}

#[test]
fn terminus_is_the_final_position() {
    let game = fools_mate();

    assert_that!(game[Terminus], eq(game.position()));
    assert_that!(
        game[Terminus].at(h4),
        some(eq(Piece { color: Color::Black, role: Role::Queen }))
    );
}

#[test]
fn terminus_minus_one_is_the_board_before_the_mating_move() {
    let game = fools_mate();

    assert_that!(game[Terminus - 1].at(h4), none());
    assert_that!(
        game[Terminus - 1].at(d8),
        some(eq(Piece { color: Color::Black, role: Role::Queen }))
    );
    assert_that!(game[Terminus - 1], eq(game[3]));
}

#[test]
fn symbolic_indices_subtract_like_numbers() {
    let game = fools_mate();

    assert_that!(game[Terminus - 1 - 1], eq(game[2]));
    assert_that!(game[Terminus - 4], eq(Position::default()));
}

#[test]
fn position_at_checks_both_ends() {
    let game = fools_mate();

    assert_that!(game.position_at(Terminus), some(eq(game.position())));
    assert_that!(game.position_at(Terminus - 5), none());
}

#[test]
#[should_panic]
fn indexing_before_the_start_panics_like_a_slice() {
    let game = fools_mate();
    let _ = game[Terminus - 5];
}

#[test]
#[should_panic]
fn indexing_past_the_end_panics_like_a_slice() {
    let game = ruy_lopez_start();
    let _ = game[6];
}

#[test]
fn undo_drops_the_last_cache_entry() {
    let game = ruy_lopez_start();

    let undone = game.undo();

    assert_that!(undone.plies(), eq(4));
    assert_that!(undone.position(), eq(game[4]));
}
