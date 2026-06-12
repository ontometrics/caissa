use caissa::classics::fools_mate;
use caissa::notation::*;
use caissa::{Color, Ending, Game, Mode, Piece, Position, Rejected, Role};
use googletest::prelude::*;

/// One rule covers pins, moving into check, and ignoring check: the
/// resulting position may not leave your king attacked.
mod check_rules {
    use super::*;

    #[test]
    fn moving_into_check_is_rejected() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(e8, Piece::black(Role::Rook));

        let result = board.play("e1e2");

        assert_that!(result, err(eq(&Rejected::IntoCheck { king: e2 })));
    }

    #[test]
    fn stepping_out_of_the_line_of_fire_is_fine() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(e8, Piece::black(Role::Rook));

        assert_that!(board.in_check(Color::White), eq(true));
        assert_that!(board.play("e1d1"), ok(anything()));
    }

    #[test]
    fn a_pinned_piece_cannot_move() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(e2, Piece::white(Role::Rook))
            .with(e8, Piece::black(Role::Rook));

        let result = board.play("e2a2");

        assert_that!(result, err(eq(&Rejected::IntoCheck { king: e1 })));
    }

    #[test]
    fn a_pinned_piece_can_slide_along_the_pin() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(e2, Piece::white(Role::Rook))
            .with(e8, Piece::black(Role::Rook));

        assert_that!(board.play("e2e8"), ok(anything()));
    }

    #[test]
    fn ignoring_check_is_rejected() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(a1, Piece::white(Role::Rook))
            .with(e8, Piece::black(Role::Rook));

        let result = board.play("a1b1");

        assert_that!(result, err(eq(&Rejected::IntoCheck { king: e1 })));
    }
}

/// Mode is derived from the board, never stored: mate, stalemate, or
/// still playing.
mod endings {
    use super::*;

    #[test]
    fn fools_mate_has_been_played() {
        let game = fools_mate();

        assert_that!(
            game.mode(),
            eq(Mode::Played(Ending::Checkmate { winner: Color::Black }))
        );
        assert_that!(game.mode(), eq(game[Terminus].mode()));
    }

    #[test]
    fn check_with_an_escape_is_still_playing() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(e8, Piece::black(Role::Rook));

        assert_that!(board.in_check(Color::White), eq(true));
        assert_that!(board.mode(), eq(Mode::Playing));
    }

    #[test]
    fn no_moves_without_check_is_stalemate() {
        let board = Position::empty(Color::Black)
            .with(a8, Piece::black(Role::King))
            .with(c7, Piece::white(Role::Queen));

        assert_that!(board.in_check(Color::Black), eq(false));
        assert_that!(board.mode(), eq(Mode::Played(Ending::Stalemate)));
    }

    #[test]
    fn a_game_born_from_a_terminal_position_is_already_played() {
        let board = Position::empty(Color::Black)
            .with(a8, Piece::black(Role::King))
            .with(c7, Piece::white(Role::Queen));

        let game = Game::from_position(board);

        assert_that!(game.mode(), eq(Mode::Played(Ending::Stalemate)));
        assert_that!(
            game.apply("a8a7"),
            err(eq(&Rejected::GameOver { ending: Ending::Stalemate }))
        );
    }
}

/// There is no position after the end of a game — and undo brings
/// the game back to life without recomputation.
mod after_the_end {
    use super::*;

    #[test]
    fn a_played_game_rejects_every_action_without_move_checking() {
        let game = fools_mate();

        let result = game.apply("a2a3");

        assert_that!(
            result,
            err(eq(&Rejected::GameOver {
                ending: Ending::Checkmate { winner: Color::Black }
            }))
        );
    }

    #[test]
    fn the_position_itself_still_rejects_on_the_merits() {
        // A position is memoryless — no mode, no GameOver. The mated side's
        // actions all fail individually instead.
        let mated = fools_mate()[Terminus];

        let result = mated.play("a2a3");

        assert_that!(result, err(eq(&Rejected::IntoCheck { king: e1 })));
    }

    #[test]
    fn undo_resumes_playing() {
        let game = fools_mate();

        let undone = game.undo();

        assert_that!(undone.mode(), eq(Mode::Playing));
        assert_that!(undone.apply("d8h4"), ok(anything()));
    }
}

/// Things a position can tell you without storing them.
mod derived_queries {
    use super::*;

    #[test]
    fn a_position_with_one_legal_action_forces_it() {
        let board = Position::empty(Color::White)
            .with(a1, Piece::white(Role::King))
            .with(b8, Piece::black(Role::Rook));

        // b1 and b2 are covered by the rook; only a1a2 remains.
        assert_that!(
            board.forced(),
            some(eq(caissa::Action::Move { from: a1, to: a2 }))
        );
        assert_that!(Position::default().forced(), none());
    }

    #[test]
    fn legal_actions_shrink_under_check() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(e8, Piece::black(Role::Rook));

        // Every legal action must be a king move off the e-file.
        let escapes: Vec<_> = board.legal_actions().collect();

        assert_that!(escapes.len(), eq(4)); // d1, d2, f1, f2
        for action in escapes {
            assert_that!(board.play(action), ok(anything()));
        }
    }
}
