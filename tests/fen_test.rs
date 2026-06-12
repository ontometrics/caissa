use caissa::classics::{fools_mate, italian, opera_game};
use caissa::{Game, Position, Rejected};
use googletest::prelude::*;

/// FEN out: the six fields, with counters derived from the log when a
/// Game writes them and defaulted when a bare Position must.
mod writing {
    use super::*;

    #[test]
    fn the_start_is_the_canonical_fen() {
        assert_that!(
            Position::default().fen().as_str(),
            eq("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        );
    }

    #[test]
    fn a_game_supplies_the_true_counters() {
        let game = Game::new().apply("e4").unwrap();

        assert_that!(
            game.fen().as_str(),
            eq("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")
        );

        let quiet = Game::new()
            .apply("Nf3")
            .and_then(|g| g.apply("Nf6"))
            .unwrap();
        assert_that!(quiet.fen().ends_with(" 2 2"), eq(true));
    }

    #[test]
    fn spent_rights_disappear_from_the_castling_field() {
        let castled = italian().play("e1g1").unwrap();

        assert_that!(castled.fen().contains(" kq "), eq(true));
    }

    #[test]
    fn the_opera_games_grave_is_well_marked() {
        assert_that!(
            opera_game().fen().as_str(),
            eq("1n1Rkb1r/p4ppp/4q3/4p1B1/4P3/8/PPP2PPP/2K5 b k - 1 17")
        );
    }
}

/// FEN in: a position read back is the position written, and garbage
/// rejects as data.
mod reading {
    use super::*;

    #[test]
    fn the_canonical_start_reads_back_as_default() {
        let read =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
                .unwrap();

        assert_that!(read, eq(Position::default()));
    }

    #[test]
    fn every_position_of_a_classic_round_trips() {
        let game = fools_mate();

        for ply in 0..=game.plies() {
            let position = game[ply];
            assert_that!(Position::from_fen(&position.fen()), ok(eq(&position)));
        }
    }

    #[test]
    fn counters_are_accepted_but_not_state() {
        let read =
            Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 37 14")
                .unwrap();

        assert_that!(read, eq(Position::default()));
    }

    #[test]
    fn malformed_fens_reject_as_data() {
        for bad in [
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP w KQkq -", // seven ranks
            "rnbqkbnr/pppppppp/9/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -", // rank overflows
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNX w KQkq -", // X is not a piece
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq -", // x is not a side
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQxq -", // x is not a right
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq z9", // z9 is not a square
            "not a fen at all",
        ] {
            assert_that!(
                Position::from_fen(bad),
                err(eq(&Rejected::Unparseable(bad.to_string())))
            );
        }
    }
}
