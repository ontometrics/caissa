use caissa::notation::*;
use caissa::{Action, Color, Piece, Position, Rejected, Role};
use googletest::prelude::*;

/// The `>>` operator is the arrow we have: `from >> to` builds an
/// action, and a role on the end turns it into a promotion.
mod building_actions {
    use super::*;

    #[test]
    fn shr_builds_an_action_from_two_squares() {
        assert_that!(e2 >> e4, eq(Action::Move { from: e2, to: e4 }));
    }

    #[test]
    fn shr_with_a_role_makes_it_a_promotion() {
        assert_that!(
            e7 >> e8 >> queen,
            eq(Action::Promote { from: e7, to: e8, into: Role::Queen })
        );
    }
}

/// Adding an action to a position applies it; Results chain, and a
/// rejection short-circuits the rest of the line.
mod board_arithmetic {
    use super::*;

    #[test]
    fn an_opening_in_operator_notation() {
        let position = (Position::default() + (e2 >> e4) + (e7 >> e5) + (g1 >> f3)).unwrap();

        assert_that!(
            position.at(f3),
            some(eq(Piece { color: Color::White, role: Role::Knight }))
        );
        assert_that!(position.turn(), eq(Color::Black));
    }

    #[test]
    fn promotion_reads_like_notation() {
        let board = Position::empty(Color::White)
            .with(h7, Piece { color: Color::White, role: Role::Pawn });

        let promoted = (board + (h7 >> h8 >> knight)).unwrap();

        assert_that!(
            promoted.at(h8),
            some(eq(Piece { color: Color::White, role: Role::Knight }))
        );
    }

    #[test]
    fn a_rejection_short_circuits_the_chain() {
        let result = Position::default() + (e7 >> e5) + (e5 >> e4);

        assert_that!(
            result,
            err(eq(&Rejected::NotYourTurn {
                piece: Piece { color: Color::Black, role: Role::Pawn }
            }))
        );
    }
}
