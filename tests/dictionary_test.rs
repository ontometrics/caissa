//! Snapshot is the default: every position is already a value, so a
//! dictionary of boards is one fold away — and tallies are a monoid, so
//! dictionaries built from different corpora merge.

use std::collections::HashMap;

use caissa::classics::{fools_mate, opera_game, ruy_lopez};
use caissa::{Color, Ending, Game, Mode, Position};
use googletest::prelude::*;

/// [white wins, undecided, black wins] — component-wise addition makes
/// this a monoid, which is what lets dictionaries merge.
type Tally = [u32; 3];

fn credit(game: &Game) -> usize {
    match game.mode() {
        Mode::Played(Ending::Checkmate {
            winner: Color::White,
        }) => 0,
        Mode::Played(Ending::Checkmate {
            winner: Color::Black,
        }) => 2,
        _ => 1,
    }
}

fn dictionary(games: &[Game]) -> HashMap<Position, Tally> {
    let mut tallies: HashMap<Position, Tally> = HashMap::new();
    for game in games {
        let bucket = credit(game);
        for ply in 0..=game.plies() {
            // Keyed by repetition_key so transpositions merge — the
            // resolution dial of the dictionary is the choice of key.
            tallies.entry(game[ply].repetition_key()).or_default()[bucket] += 1;
        }
    }
    tallies
}

#[test]
fn a_dictionary_of_positions_is_a_fold_over_games() {
    let book = dictionary(&[opera_game(), ruy_lopez(), fools_mate()]);

    // Every game starts at the start: one White win (Opera), one game
    // still open (Ruy Lopez), one Black win (Fool's Mate).
    let start = Position::default().repetition_key();
    assert_that!(book.get(&start), some(eq(&[1, 1, 1])));

    // Two of the three open 1. e4 — the same key, met in different games.
    let after_e4 = Position::default().play("e4").unwrap().repetition_key();
    assert_that!(book.get(&after_e4), some(eq(&[1, 1, 0])));
}

#[test]
fn many_roads_one_board_the_maze_collapses() {
    let game_of = |line: &[&str]| {
        line.iter()
            .try_fold(Game::new(), |game, action| game.apply(*action))
            .unwrap()
    };
    // The same opening by two move orders: king's pawn first, or
    // knight first. One board, two roads.
    let kings_road = game_of(&["e2e4", "e7e5", "g1f3"]);
    let knights_road = game_of(&["g1f3", "e7e5", "e2e4"]);

    // Structurally they differ: the knight-first road ends on a double
    // push and carries a (dead) en-passant residue the other lacks...
    assert_that!(kings_road.position() == knights_road.position(), eq(false));
    // ...which repetition_key launders: the Markov state is the same.
    assert_that!(
        kings_road.position().repetition_key(),
        eq(knights_road.position().repetition_key())
    );

    // So the dictionary holds ONE entry for that board — forward
    // probabilities never depend on the road taken.
    let book = dictionary(&[kings_road.clone(), knights_road]);
    let key = kings_road.position().repetition_key();
    assert_that!(book.get(&key), some(eq(&[0, 2, 0])));
}

#[test]
fn tallies_merge_because_they_are_a_monoid() {
    let mut merged = dictionary(&[opera_game()]);
    for (key, tally) in dictionary(&[fools_mate(), ruy_lopez()]) {
        let entry = merged.entry(key).or_default();
        for (slot, count) in tally.into_iter().enumerate() {
            entry[slot] += count;
        }
    }

    assert_that!(
        merged,
        eq(&dictionary(&[opera_game(), fools_mate(), ruy_lopez()]))
    );
}
