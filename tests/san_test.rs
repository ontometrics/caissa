use caissa::notation::*;
use caissa::{Action, Color, Ending, Game, Mode, Origin, Piece, Position, Rejected, Role, San};
use googletest::prelude::*;

fn white(role: Role) -> Piece {
    Piece { color: Color::White, role }
}

fn black(role: Role) -> Piece {
    Piece { color: Color::Black, role }
}

mod resolution {
    use super::*;

    #[test]
    fn the_original_dream() {
        let game = ["e4", "e5", "Nf3", "Nc6", "Bb5"]
            .into_iter()
            .try_fold(Game::new(), |game, san| game.apply(san))
            .unwrap();

        assert_that!(game.position().at(b5), some(eq(white(Role::Bishop))));
        assert_that!(game.plies(), eq(5));
    }

    #[test]
    fn uci_and_san_mix_freely() {
        let position = Position::default()
            .play("e2e4")
            .and_then(|p| p.play("e5"))
            .unwrap();

        assert_that!(position.at(e5), some(eq(black(Role::Pawn))));
    }

    #[test]
    fn an_undescribed_origin_resolves_when_unique() {
        let position = Position::default().play("Nf3").unwrap();

        assert_that!(position.at(f3), some(eq(white(Role::Knight))));
    }

    #[test]
    fn no_match_is_data() {
        let result = Position::default().play("Nf6");

        assert_that!(
            result,
            err(eq(&Rejected::NoMatch {
                san: San::Move { role: Role::Knight, origin: Origin::Anywhere, to: f6 }
            }))
        );
    }
}

mod disambiguation {
    use super::*;

    #[test]
    fn an_ambiguous_san_returns_the_candidates() {
        let board = Position::empty(Color::White)
            .with(a1, white(Role::Rook))
            .with(h1, white(Role::Rook));

        let result = board.play("Re1");

        assert_that!(
            result,
            err(eq(&Rejected::AmbiguousSan {
                candidates: vec![
                    Action::Move { from: a1, to: e1 },
                    Action::Move { from: h1, to: e1 },
                ]
            }))
        );
    }

    #[test]
    fn a_file_settles_the_ambiguity() {
        let board = Position::empty(Color::White)
            .with(a1, white(Role::Rook))
            .with(h1, white(Role::Rook));

        let position = board.play("Rae1").unwrap();

        assert_that!(position.at(e1), some(eq(white(Role::Rook))));
        assert_that!(position.at(a1), none());
        assert_that!(position.at(h1), some(eq(white(Role::Rook))));
    }

    #[test]
    fn a_rank_settles_it_when_a_file_cannot() {
        let board = Position::empty(Color::White)
            .with(a1, white(Role::Rook))
            .with(a5, white(Role::Rook));

        let position = board.play("R1a3").unwrap();

        assert_that!(position.at(a3), some(eq(white(Role::Rook))));
        assert_that!(position.at(a5), some(eq(white(Role::Rook))));
    }

    #[test]
    fn a_full_square_settles_what_neither_could() {
        // File e is ambiguous (e2, e6), rank 4 is ambiguous (a4, h4) —
        // only the full square names the mover.
        let board = Position::empty(Color::White)
            .with(e2, white(Role::Queen))
            .with(e6, white(Role::Queen))
            .with(a4, white(Role::Queen))
            .with(h4, white(Role::Queen));

        assert_that!(board.play("Qee4"), err(anything()));
        assert_that!(board.play("Q4e4"), err(anything()));

        let position = board.play("Qa4e4").unwrap();

        assert_that!(position.at(e4), some(eq(white(Role::Queen))));
        assert_that!(position.at(a4), none());
    }
}

mod promotion {
    use super::*;

    #[test]
    fn promotion_san_resolves_to_the_promote_action() {
        let board = Position::empty(Color::White).with(h7, white(Role::Pawn));

        let position = board.play("h8=Q").unwrap();

        assert_that!(position.at(h8), some(eq(white(Role::Queen))));
    }

    #[test]
    fn a_capturing_underpromotion_reads_like_a_game_score() {
        let board = Position::empty(Color::White)
            .with(h7, white(Role::Pawn))
            .with(g8, black(Role::Rook));

        let position = board.play("hxg8=N").unwrap();

        assert_that!(position.at(g8), some(eq(white(Role::Knight))));
    }
}

mod castling_and_suffixes {
    use super::*;

    #[test]
    fn castles_desugar_to_the_kings_two_square_move() {
        let italian = ["e4", "e5", "Nf3", "Nf6", "Bc4", "Bc5"]
            .into_iter()
            .try_fold(Position::default(), Position::play)
            .unwrap();

        let castled = italian.play("O-O").unwrap();

        assert_that!(castled.at(g1), some(eq(white(Role::King))));
        assert_that!(castled.at(f1), some(eq(white(Role::Rook))));
    }

    #[test]
    fn check_and_mate_suffixes_are_tolerated() {
        let game = ["f3", "e5", "g4", "Qh4#"]
            .into_iter()
            .try_fold(Game::new(), |game, san| game.apply(san))
            .unwrap();

        assert_that!(
            game.mode(),
            eq(Mode::Played(Ending::Checkmate { winner: Color::Black }))
        );
    }
}
