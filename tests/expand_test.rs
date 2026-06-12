//! The interpreter, observed: every action expands from a prototype into
//! primitive edits before anything touches the board.

use caissa::notation::*;
use caissa::{Color, Edit, Piece, Position, Role, expand};
use googletest::prelude::*;

fn white(role: Role) -> Piece {
    Piece { color: Color::White, role }
}

fn black(role: Role) -> Piece {
    Piece { color: Color::Black, role }
}

fn fold(line: &[&str]) -> Position {
    line.iter()
        .try_fold(Position::default(), |position, action| position.play(*action))
        .unwrap()
}

mod simple_prototypes {
    use super::*;

    #[test]
    fn a_quiet_move_is_two_edits() {
        let change = expand(Position::default(), "e2e4".parse().unwrap()).unwrap();

        assert_that!(
            change.edits,
            eq(&vec![
                Edit::Lift(e2),
                Edit::Place(e4, white(Role::Pawn)),
            ])
        );
        assert_that!(change.passant, some(eq(e3)));
    }

    #[test]
    fn a_capture_lifts_the_victim_first() {
        let board = fold(&["e2e4", "d7d5"]);

        let change = expand(board, "e4d5".parse().unwrap()).unwrap();

        assert_that!(
            change.edits,
            eq(&vec![
                Edit::Lift(d5),
                Edit::Lift(e4),
                Edit::Place(d5, white(Role::Pawn)),
            ])
        );
    }
}

mod compound_prototypes {
    use super::*;

    #[test]
    fn castling_expands_from_its_prototype() {
        let board = fold(&["e2e4", "e7e5", "g1f3", "g8f6", "f1c4", "f8c5"]);

        let change = expand(board, "e1g1".parse().unwrap()).unwrap();

        assert_that!(
            change.edits,
            eq(&vec![
                Edit::Lift(e1),
                Edit::Lift(h1),
                Edit::Place(g1, white(Role::King)),
                Edit::Place(f1, white(Role::Rook)),
            ])
        );
    }

    #[test]
    fn en_passant_lifts_the_passed_pawn() {
        let board = fold(&["e2e4", "a7a6", "e4e5", "d7d5"]);

        let change = expand(board, "e5d6".parse().unwrap()).unwrap();

        assert_that!(
            change.edits,
            eq(&vec![
                Edit::Lift(d5),
                Edit::Lift(e5),
                Edit::Place(d6, white(Role::Pawn)),
            ])
        );
    }

    #[test]
    fn promotion_places_what_the_pawn_becomes() {
        let board = Position::empty(Color::White)
            .with(h7, white(Role::Pawn))
            .with(g8, black(Role::Rook));

        let change = expand(board, "h7g8q".parse().unwrap()).unwrap();

        assert_that!(
            change.edits,
            eq(&vec![
                Edit::Lift(g8),
                Edit::Lift(h7),
                Edit::Place(g8, white(Role::Queen)),
            ])
        );
    }
}
