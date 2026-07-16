//! A thinking player: search is the fold run speculatively. Tests stay on
//! sparse positions at shallow depth — the interpreter-priced movegen is
//! ruinous on a full board (the vision's reserved fast path), but cheap
//! when most squares are empty.

use caissa::notation::*;
use caissa::play::{Minimax, Player, material};
use caissa::{Action, Color, Ending, Game, Mode, Piece, Position, Role};
use googletest::prelude::*;

fn white(role: Role) -> Piece {
    Piece {
        color: Color::White,
        role,
    }
}

fn black(role: Role) -> Piece {
    Piece {
        color: Color::Black,
        role,
    }
}

/// Ŵ, the cheap estimate — the first rung of the evaluator ladder.
mod evaluation {
    use super::*;

    #[test]
    fn material_is_symmetric_at_the_start() {
        assert_that!(material(Position::default()), eq(0));
    }

    #[test]
    fn material_counts_from_the_movers_view() {
        let board = Position::empty(Color::White)
            .with(e1, white(Role::King))
            .with(e8, black(Role::King))
            .with(d1, white(Role::Queen));

        assert_that!(material(board), eq(900)); // White, to move, is up a queen
        assert_that!(
            material(Position::empty(Color::Black).with(d1, white(Role::Queen))),
            eq(-900)
        );
    }
}

/// Searching the fold finds the move material alone would, and the mate
/// material can't see.
mod tactics {
    use super::*;

    #[test]
    fn it_takes_a_hanging_queen() {
        // White rook on a1, Black queen undefended on a8.
        let board = Position::empty(Color::White)
            .with(e1, white(Role::King))
            .with(e8, black(Role::King))
            .with(a1, white(Role::Rook))
            .with(a8, black(Role::Queen));

        let move_chosen = Minimax::new(1, material).choose(&Game::from_position(board));

        assert_that!(move_chosen, eq(Action::Move { from: a1, to: a8 }));
    }

    #[test]
    fn it_finds_mate_in_one() {
        // Kh6 + Qa7 vs lone Kh8: Qg7# and Qh7# both mate.
        let board = Position::empty(Color::White)
            .with(h6, white(Role::King))
            .with(a7, white(Role::Queen))
            .with(h8, black(Role::King));
        let game = Game::from_position(board);

        let move_chosen = Minimax::new(1, material).choose(&game);
        let after = game.apply(move_chosen).unwrap();

        // Robust to which mate it picks: assert the move actually mates.
        assert_that!(
            after.mode(),
            eq(Mode::Played(Ending::Checkmate {
                winner: Color::White
            }))
        );
    }

    #[test]
    fn deeper_search_is_deterministic() {
        let board = Position::empty(Color::White)
            .with(e1, white(Role::King))
            .with(e8, black(Role::King))
            .with(a1, white(Role::Rook))
            .with(a8, black(Role::Queen));
        let game = Game::from_position(board);

        let once = Minimax::new(2, material).choose(&game);
        let again = Minimax::new(2, material).choose(&game);

        assert_that!(once, eq(again));
    }
}
