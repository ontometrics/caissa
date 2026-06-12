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
        Mode::Played(Ending::Checkmate { winner: Color::White }) => 0,
        Mode::Played(Ending::Checkmate { winner: Color::Black }) => 2,
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
fn tallies_merge_because_they_are_a_monoid() {
    let mut merged = dictionary(&[opera_game()]);
    for (key, tally) in dictionary(&[fools_mate(), ruy_lopez()]) {
        let entry = merged.entry(key).or_default();
        for (slot, count) in tally.into_iter().enumerate() {
            entry[slot] += count;
        }
    }

    assert_that!(merged, eq(&dictionary(&[opera_game(), fools_mate(), ruy_lopez()])));
}
