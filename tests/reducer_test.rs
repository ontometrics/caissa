use caissa::{Action, Color, Game, Piece, Position, Rejected, Role, Square};
use googletest::prelude::*;

fn square(s: &str) -> Square {
    s.parse().unwrap()
}

fn piece(color: Color, role: Role) -> Piece {
    Piece { color, role }
}

/// The thesis: moves are pure transitions, composition is the API —
/// values in, values out, earlier positions intact.
mod pure_transitions {
    use super::*;

    #[test]
    fn an_opening_is_a_fold() {
        let position = ["e2e4", "e7e5", "g1f3"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        assert_that!(
            position.at(square("f3")),
            some(eq(piece(Color::White, Role::Knight)))
        );
        assert_that!(position.turn(), eq(Color::Black));
    }

    #[test]
    fn earlier_positions_survive_later_moves() {
        let start = Position::default();
        let after = start.play("e2e4").unwrap();

        assert_that!(start.at(square("e2")), some(eq(piece(Color::White, Role::Pawn))));
        assert_that!(after.at(square("e2")), none());
        assert_that!(after.at(square("e4")), some(eq(piece(Color::White, Role::Pawn))));
    }

    #[test]
    fn a_capture_needs_no_annotation() {
        let position = ["e2e4", "d7d5", "e4d5"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        assert_that!(
            position.at(square("d5")),
            some(eq(piece(Color::White, Role::Pawn)))
        );
    }

    #[test]
    fn square_pairs_are_actions_too() {
        let position = Position::default()
            .play((square("e2"), square("e4")))
            .unwrap();

        assert_that!(
            position.at(square("e4")),
            some(eq(piece(Color::White, Role::Pawn)))
        );
    }
}

/// Errors are data: every refusal names what was wrong, and where.
mod rejections {
    use super::*;

    #[test]
    fn moving_out_of_turn_is_rejected() {
        let result = Position::default().play("e7e5");

        assert_that!(
            result,
            err(eq(&Rejected::NotYourTurn {
                piece: piece(Color::Black, Role::Pawn)
            }))
        );
    }

    #[test]
    fn a_blocked_rook_cannot_reach() {
        let result = Position::default().play("a1a3");

        assert_that!(
            result,
            err(eq(&Rejected::CannotReach {
                from: square("a1"),
                to: square("a3")
            }))
        );
    }

    #[test]
    fn a_knight_cannot_move_like_a_rook() {
        let result = Position::default().play("g1g3");

        assert_that!(
            result,
            err(eq(&Rejected::CannotReach {
                from: square("g1"),
                to: square("g3")
            }))
        );
    }

    #[test]
    fn an_empty_square_is_rejected() {
        let result = Position::default().play("e4e5");

        assert_that!(result, err(eq(&Rejected::EmptySquare { from: square("e4") })));
    }

    #[test]
    fn gibberish_is_rejected_as_data() {
        let result = Position::default().play("xyzzy");

        assert_that!(result, err(eq(&Rejected::Unparseable("xyzzy".to_string()))));
    }

    #[test]
    fn capturing_your_own_piece_is_rejected() {
        let result = Position::default().play(Action::Move {
            from: square("e1"),
            to: square("d1"),
        });

        assert_that!(result, err(eq(&Rejected::OwnPieceAt { to: square("d1") })));
    }
}

/// The one move where from–to underdetermines intent, so it gets its
/// own action variant — and its own rules.
mod promotion {
    use super::*;

    #[test]
    fn a_pawn_on_the_last_rank_demands_promotion() {
        let position =
            Position::empty(Color::White).with(square("h7"), piece(Color::White, Role::Pawn));

        let result = position.play("h7h8");

        assert_that!(
            result,
            err(eq(&Rejected::NeedsPromotion {
                from: square("h7"),
                to: square("h8")
            }))
        );
    }

    #[test]
    fn promotion_states_what_the_pawn_becomes() {
        let position =
            Position::empty(Color::White).with(square("h7"), piece(Color::White, Role::Pawn));

        let promoted = position.play("h7h8q").unwrap();

        assert_that!(
            promoted.at(square("h8")),
            some(eq(piece(Color::White, Role::Queen)))
        );
    }

    #[test]
    fn underpromotion_to_a_knight_works() {
        let position =
            Position::empty(Color::White).with(square("h7"), piece(Color::White, Role::Pawn));

        let promoted = position.play("h7h8n").unwrap();

        assert_that!(
            promoted.at(square("h8")),
            some(eq(piece(Color::White, Role::Knight)))
        );
    }

    #[test]
    fn promoting_a_non_promotion_move_is_rejected() {
        let result = Position::default().play("e2e4q");

        assert_that!(
            result,
            err(eq(&Rejected::NotAPromotion {
                from: square("e2"),
                to: square("e4")
            }))
        );
    }

    #[test]
    fn a_pawn_cannot_promote_to_a_king() {
        let position =
            Position::empty(Color::White).with(square("h7"), piece(Color::White, Role::Pawn));

        let result = position.play(Action::Promote {
            from: square("h7"),
            to: square("h8"),
            into: Role::King,
        });

        assert_that!(
            result,
            err(eq(&Rejected::InvalidPromotion { into: Role::King }))
        );
    }
}

/// A game is its log: replay is a fold, undo is a prefix.
mod the_game_log {
    use super::*;

    #[test]
    fn a_game_is_its_log() {
        let game = Game::new()
            .apply("e2e4")
            .and_then(|game| game.apply("e7e5"))
            .unwrap();

        assert_that!(game.log().len(), eq(2));
        assert_that!(
            Game::replay(Position::default(), game.log().to_vec()),
            ok(eq(&game.position()))
        );
    }

    #[test]
    fn undo_is_replaying_a_prefix() {
        let game = Game::new()
            .apply("e2e4")
            .and_then(|game| game.apply("e7e5"))
            .unwrap();

        let undone = game.undo();

        assert_that!(undone.log().len(), eq(1));
        assert_that!(
            undone.position(),
            eq(Game::new().apply("e2e4").unwrap().position())
        );
        assert_that!(game.log().len(), eq(2));
    }
}
