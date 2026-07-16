use caissa::classics::italian;
use caissa::notation::*;
use caissa::{Color, Piece, Position, Rejected, Role, Wing};
use googletest::prelude::*;

/// The king's two-square move, interpreted: both pieces land where
/// they should, and the rights are spent.
mod castling {
    use super::*;

    #[test]
    fn castling_is_the_kings_two_square_move() {
        let castled = italian().play("e1g1").unwrap();

        assert_that!(castled.at(g1), some(eq(Piece::white(Role::King))));
        assert_that!(castled.at(f1), some(eq(Piece::white(Role::Rook))));
        assert_that!(castled.at(e1), none());
        assert_that!(castled.at(h1), none());
    }

    #[test]
    fn castling_spends_both_rights() {
        let castled = italian().play("e1g1").unwrap();

        assert_that!(castled.may_castle(Color::White, Wing::King), eq(false));
        assert_that!(castled.may_castle(Color::White, Wing::Queen), eq(false));
        assert_that!(castled.may_castle(Color::Black, Wing::King), eq(true));
    }

    #[test]
    fn queenside_castling_works_too() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(a1, Piece::white(Role::Rook));

        let castled = board.play("e1c1").unwrap();

        assert_that!(castled.at(c1), some(eq(Piece::white(Role::King))));
        assert_that!(castled.at(d1), some(eq(Piece::white(Role::Rook))));
        assert_that!(castled.at(a1), none());
    }
}

/// Rights only ever shrink: kings forfeit both wings, rooks only
/// their own — and nothing restores them.
mod forfeits {
    use super::*;

    #[test]
    fn a_wandering_king_forfeits_castling_forever() {
        let board = ["e1e2", "a7a6", "e2e1", "a6a5"]
            .into_iter()
            .try_fold(italian(), Position::play)
            .unwrap();

        let result = board.play("e1g1");

        assert_that!(
            result,
            err(eq(&Rejected::CastlingForfeited { wing: Wing::King }))
        );
    }

    #[test]
    fn a_moved_rook_forfeits_only_its_own_wing() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(a1, Piece::white(Role::Rook))
            .with(h1, Piece::white(Role::Rook))
            .with(e8, Piece::black(Role::King));

        let round_trip = ["h1g1", "e8d8", "g1h1", "d8e8"]
            .into_iter()
            .try_fold(board, Position::play)
            .unwrap();

        assert_that!(round_trip.may_castle(Color::White, Wing::King), eq(false));
        assert_that!(round_trip.may_castle(Color::White, Wing::Queen), eq(true));
        assert_that!(
            round_trip.play("e1g1"),
            err(eq(&Rejected::CastlingForfeited { wing: Wing::King }))
        );
        assert_that!(round_trip.play("e1c1"), ok(anything()));
    }
}

/// Castling refused: out of, through, or into check — or simply
/// blocked.
mod denials {
    use super::*;

    #[test]
    fn castling_through_check_is_rejected() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(h1, Piece::white(Role::Rook))
            .with(f8, Piece::black(Role::Rook));

        let result = board.play("e1g1");

        assert_that!(result, err(eq(&Rejected::IntoCheck { king: f1 })));
    }

    #[test]
    fn castling_out_of_check_is_rejected() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(h1, Piece::white(Role::Rook))
            .with(e8, Piece::black(Role::Rook));

        let result = board.play("e1g1");

        assert_that!(result, err(eq(&Rejected::IntoCheck { king: e1 })));
    }

    #[test]
    fn castling_with_pieces_between_cannot_reach() {
        let result = Position::default().play("e1g1");

        assert_that!(result, err(eq(&Rejected::CannotReach { from: e1, to: g1 })));
    }
}

/// The position's other memory: a one-ply window onto the square the
/// double push skipped.
mod en_passant {
    use super::*;

    #[test]
    fn en_passant_takes_the_pawn_that_passed() {
        let board = ["e2e4", "a7a6", "e4e5", "d7d5"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        let captured = board.play("e5d6").unwrap();

        assert_that!(captured.at(d6), some(eq(Piece::white(Role::Pawn))));
        assert_that!(captured.at(d5), none());
    }

    #[test]
    fn the_en_passant_window_closes_after_one_ply() {
        let board = ["e2e4", "a7a6", "e4e5", "d7d5", "h2h3", "a6a5"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        let result = board.play("e5d6");

        assert_that!(result, err(eq(&Rejected::CannotReach { from: e5, to: d6 })));
    }

    #[test]
    fn a_double_push_records_the_skipped_square() {
        let opened = Position::default().play("e2e4").unwrap();
        assert_that!(opened.passant(), some(eq(e3)));

        let declined = opened.play("a7a6").unwrap();
        assert_that!(declined.passant(), none());
    }
}
