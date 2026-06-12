use caissa::notation::*;
use caissa::{Color, DrawReason, Ending, Game, Mode, Piece, Position, Rejected, Role};
use googletest::prelude::*;

/// Knights out and back: every round of four plies returns to the
/// starting position, adding one occurrence.
fn shuffled(rounds: usize) -> Game {
    (0..rounds).fold(Game::new(), |game, _| {
        ["g1f3", "g8f6", "f3g1", "f6g8"]
            .into_iter()
            .try_fold(game, |game, action| game.apply(action))
            .unwrap()
    })
}

/// K+R vs K+R, both rooks touring disjoint regions in cycles of coprime
/// length (12 and 33), so the combined state first repeats after 264
/// plies — every ply quiet, every position fresh. The engine validates
/// each move; the test only scripts them.
fn rook_marathon(plies: usize) -> Game {
    let white_cycle = [
        "b1", "c1", "d1", "e1", "f1", "g1", "g2", "f2", "e2", "d2", "c2", "b2",
    ];
    let black_cycle = [
        "f8", "e8", "d8", "c8", "b8", "b7", "c7", "d7", "e7", "f7", "g7", "h7", "h6", "g6", "f6",
        "e6", "d6", "c6", "b6", "b5", "c5", "d5", "e5", "f5", "g5", "h5", "h4", "g4", "e4", "d4",
        "c4", "b4", "f4",
    ];
    let board = Position::empty(Color::White)
        .with(a1, Piece::white(Role::King))
        .with(b1, Piece::white(Role::Rook))
        .with(h8, Piece::black(Role::King))
        .with(f8, Piece::black(Role::Rook));
    let mut game = Game::from_position(board);
    let (mut white_at, mut black_at) = (0, 0);
    for ply in 0..plies {
        let action = if ply % 2 == 0 {
            let from = white_cycle[white_at % white_cycle.len()];
            white_at += 1;
            format!("{from}{}", white_cycle[white_at % white_cycle.len()])
        } else {
            let from = black_cycle[black_at % black_cycle.len()];
            black_at += 1;
            format!("{from}{}", black_cycle[black_at % black_cycle.len()])
        };
        game = game.apply(action.as_str()).unwrap();
    }
    game
}

/// FIDE's dead position by the material table: a board fact, so it lives
/// in position-level mode() next to mate and stalemate.
mod dead_positions {
    use super::*;

    #[test]
    fn bare_kings_cannot_mate() {
        let board = Position::empty(Color::White)
            .with(a1, Piece::white(Role::King))
            .with(h8, Piece::black(Role::King));

        assert_that!(
            board.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::InsufficientMaterial)))
        );
    }

    #[test]
    fn a_lone_minor_piece_cannot_mate() {
        let board = Position::empty(Color::White)
            .with(a1, Piece::white(Role::King))
            .with(h8, Piece::black(Role::King))
            .with(d4, Piece::white(Role::Knight));

        assert_that!(
            board.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::InsufficientMaterial)))
        );
    }

    #[test]
    fn same_shaded_lone_bishops_are_dead_opposite_are_not() {
        let kings = Position::empty(Color::White)
            .with(a1, Piece::white(Role::King))
            .with(h8, Piece::black(Role::King));

        let same_shade = kings
            .with(c1, Piece::white(Role::Bishop))
            .with(f8, Piece::black(Role::Bishop));
        assert_that!(
            same_shade.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::InsufficientMaterial)))
        );

        let opposite = kings
            .with(c1, Piece::white(Role::Bishop))
            .with(c8, Piece::black(Role::Bishop));
        assert_that!(opposite.mode(), eq(Mode::Playing));
    }

    #[test]
    fn a_rook_is_mating_material() {
        let board = Position::empty(Color::White)
            .with(a1, Piece::white(Role::King))
            .with(h8, Piece::black(Role::King))
            .with(d4, Piece::white(Role::Rook));

        assert_that!(board.mode(), eq(Mode::Playing));
    }

    #[test]
    fn the_capture_that_kills_the_material_ends_the_game() {
        let board = Position::empty(Color::White)
            .with(e1, Piece::white(Role::King))
            .with(d1, Piece::white(Role::Rook))
            .with(e8, Piece::black(Role::King))
            .with(d8, Piece::black(Role::Rook));

        let game = Game::from_position(board)
            .apply("d1d8")
            .and_then(|game| game.apply("e8d8"))
            .unwrap();

        assert_that!(
            game.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::InsufficientMaterial)))
        );
        assert_that!(
            game.apply("d8e8"),
            err(eq(&Rejected::GameOver {
                ending: Ending::Draw(DrawReason::InsufficientMaterial)
            }))
        );
    }
}

/// Repetition counts FIDE's "same position" — a domain equivalence via
/// repetition_key, not structural equality.
mod repetition {
    use super::*;

    #[test]
    fn threefold_is_armed_but_must_be_claimed() {
        let game = shuffled(2); // the start has now occurred three times

        assert_that!(game.repetitions(), eq(3));
        assert_that!(game.mode(), eq(Mode::Playing));

        let drawn = game.claim_draw().unwrap();
        assert_that!(
            drawn.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::Threefold)))
        );
    }

    #[test]
    fn two_occurrences_is_nothing_to_claim() {
        let game = shuffled(1);

        assert_that!(game.repetitions(), eq(2));
        assert_that!(game.claim_draw(), err(eq(&Rejected::NoDrawToClaim)));
    }

    #[test]
    fn an_unclaimed_threefold_keeps_the_game_playing() {
        let game = shuffled(3); // four occurrences, nobody asked

        assert_that!(game.repetitions(), eq(4));
        assert_that!(game.mode(), eq(Mode::Playing));
    }

    #[test]
    fn fivefold_arrives_by_itself() {
        let game = shuffled(4);

        assert_that!(
            game.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::Fivefold)))
        );
    }

    #[test]
    fn a_dead_en_passant_right_does_not_distinguish_positions() {
        // After 1. e4 no black pawn can take on e3: structurally the
        // position records the skipped square, but by FIDE it is the same
        // position as one without the right.
        let opened = Position::default().play("e2e4").unwrap();

        assert_that!(opened.passant(), some(eq(e3)));
        assert_that!(opened.repetition_key().passant(), none());
    }

    #[test]
    fn a_live_en_passant_right_does_distinguish() {
        let board = ["e2e4", "d7d5", "e4e5", "f7f5"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        assert_that!(board.passant(), some(eq(f6)));
        assert_that!(board.repetition_key().passant(), some(eq(f6)));
    }
}

/// The move-count rules ride the derived halfmove clock: quiet_plies
/// walks the log, nothing is stored.
mod long_quiet_games {
    use super::*;

    #[test]
    fn quiet_plies_reset_on_pawn_moves_and_captures() {
        assert_that!(shuffled(1).quiet_plies(), eq(4));
        assert_that!(Game::new().apply("e2e4").unwrap().quiet_plies(), eq(0));

        let capture = ["e2e4", "d7d5", "e4d5"]
            .into_iter()
            .try_fold(Game::new(), |game, action| game.apply(action))
            .unwrap();
        assert_that!(capture.quiet_plies(), eq(0));
    }

    #[test]
    fn fifty_quiet_moves_arm_a_claim() {
        let game = rook_marathon(100);

        assert_that!(game.quiet_plies(), eq(100));
        assert_that!(game.repetitions(), eq(1));
        assert_that!(
            game.claim_draw().unwrap().mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::FiftyMoves)))
        );
    }

    #[test]
    fn ninety_nine_quiet_plies_is_one_short() {
        let game = rook_marathon(99);

        assert_that!(game.claim_draw(), err(eq(&Rejected::NoDrawToClaim)));
    }

    #[test]
    fn seventy_five_quiet_moves_end_the_game_by_themselves() {
        let game = rook_marathon(150);

        assert_that!(
            game.mode(),
            eq(Mode::Played(Ending::Draw(DrawReason::SeventyFiveMoves)))
        );
        assert_that!(rook_marathon(149).mode(), eq(Mode::Playing));
    }
}

/// Drawn games write the result the board attests.
mod scoring {
    use super::*;

    #[test]
    fn a_drawn_game_scores_half_half() {
        let game = shuffled(4); // fivefold, automatic

        assert_that!(game.score().ends_with("1/2-1/2"), eq(true));
    }

    #[test]
    fn a_claimed_draw_scores_half_half_too() {
        let drawn = shuffled(2).claim_draw().unwrap();

        assert_that!(drawn.score().ends_with("1/2-1/2"), eq(true));
    }
}
