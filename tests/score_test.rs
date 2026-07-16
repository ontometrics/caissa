//! Emission is resolution's inverse: a game written out as a score must
//! fold back to the same game.

use caissa::classics::OPERA_GAME_PGN;
use caissa::notation::*;
use caissa::{Action, Color, Game, Piece, Position, Role, import, to_san};
use googletest::prelude::*;

/// The score as a publication prints it: numbers, suffixes, the
/// result the board attests, figurines on request.
mod newspaper {
    use super::*;

    #[test]
    fn a_score_reads_like_a_newspaper_column() {
        let game = ["f3", "e5", "g4", "Qh4#"]
            .into_iter()
            .try_fold(Game::new(), |game, san| game.apply(san))
            .unwrap();

        assert_that!(game.score().as_str(), eq("1. f3 e5 2. g4 Qh4# 0-1"));
        assert_that!(game.to_string().as_str(), eq("1. f3 e5 2. g4 Qh4# 0-1"));
    }

    #[test]
    fn an_unfinished_game_scores_with_a_star() {
        let game = Game::new().apply("e4").unwrap();

        assert_that!(game.score().as_str(), eq("1. e4 *"));
    }

    #[test]
    fn figurines_wear_the_movers_glyph() {
        let game = import(OPERA_GAME_PGN).unwrap();

        let figurines = game.figurines();
        assert_that!(figurines.ends_with("17. ♖d8# 1-0"), eq(true));
        assert_that!(figurines.contains("3. d4 ♝g4"), eq(true)); // black bishop, black glyph
    }
}

/// Emission is resolution's inverse: import(game.score()) must
/// reproduce the game.
mod round_trips {
    use super::*;

    #[test]
    fn the_opera_game_round_trips() {
        let game = import(OPERA_GAME_PGN).unwrap();

        let score = game.score();
        assert_that!(score.contains("11. Bxb5+ Nbd7 12. O-O-O Rd8"), eq(true));
        assert_that!(score.ends_with("17. Rd8# 1-0"), eq(true));

        let again = import(&score).unwrap();
        assert_that!(again.plies(), eq(game.plies()));
        assert_that!(again[Terminus], eq(game[Terminus]));
    }
}

/// Minimal disambiguation and the tricky spellings: en passant,
/// capturing underpromotion.
mod precision {
    use super::*;

    #[test]
    fn emission_disambiguates_minimally() {
        let two_files = Position::empty(Color::White)
            .with(a1, Piece::white(Role::Rook))
            .with(h1, Piece::white(Role::Rook));
        assert_that!(
            to_san(two_files, Action::Move { from: a1, to: e1 }),
            ok(eq(&"Rae1".to_string()))
        );

        let two_ranks = Position::empty(Color::White)
            .with(a1, Piece::white(Role::Rook))
            .with(a5, Piece::white(Role::Rook));
        assert_that!(
            to_san(two_ranks, Action::Move { from: a1, to: a3 }),
            ok(eq(&"R1a3".to_string()))
        );

        let alone = Position::empty(Color::White).with(a1, Piece::white(Role::Rook));
        assert_that!(
            to_san(alone, Action::Move { from: a1, to: e1 }),
            ok(eq(&"Re1".to_string()))
        );
    }

    #[test]
    fn en_passant_writes_as_a_plain_pawn_capture() {
        let board = ["e2e4", "a7a6", "e4e5", "d7d5"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        assert_that!(
            to_san(board, Action::Move { from: e5, to: d6 }),
            ok(eq(&"exd6".to_string()))
        );
    }

    #[test]
    fn a_capturing_underpromotion_writes_itself() {
        let board = Position::empty(Color::White)
            .with(h7, Piece::white(Role::Pawn))
            .with(
                g8,
                Piece {
                    color: Color::Black,
                    role: Role::Rook,
                },
            );

        assert_that!(
            to_san(
                board,
                Action::Promote {
                    from: h7,
                    to: g8,
                    into: Role::Knight
                }
            ),
            ok(eq(&"hxg8=N".to_string()))
        );
    }
}
