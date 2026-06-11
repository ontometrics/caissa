//! The full speedchess loop, deterministically: caissa's core takes time
//! as data and never reads a clock, so the only place a clock exists is
//! the watcher — the effectful driver that auto-claims a fallen flag.
//! synth-clock plays the wall clock, scheduled callbacks play the server,
//! and no test ever sleeps.

use std::sync::{Arc, Mutex};

use caissa::{Clocked, Color, Ending, Mode, Position};
use chrono::{DateTime, Duration, Utc};
use googletest::prelude::*;
use synth_clock::{Clock, Handle, SyntheticClock};

type Speedchess = Clocked<DateTime<Utc>, Duration>;

/// Schedule an auto-flag claim for the player to move, at the instant
/// their budget runs out. Returns the handle so a move that arrives in
/// time can cancel and re-arm — the whole server loop.
fn arm(clock: &dyn Clock, table: &Arc<Mutex<Speedchess>>) -> Handle {
    let falls_at = {
        let t = table.lock().unwrap();
        let mover = t.timeline().game().position().turn();
        let latest = t.timeline().latest();
        latest + t.remaining(mover, latest) + Duration::milliseconds(1)
    };
    let shared = Arc::clone(table);
    clock.schedule(
        falls_at,
        Box::new(move |now| {
            let mut t = shared.lock().unwrap();
            if let Ok(flagged) = t.claim_flag(now) {
                *t = flagged;
            }
        }),
    )
}

fn record(table: &Arc<Mutex<Speedchess>>, action: &str, at: DateTime<Utc>) {
    let mut t = table.lock().unwrap();
    let next = t.record(action, at).unwrap();
    *t = next;
}

#[test]
fn the_watcher_flags_a_player_who_never_moves() {
    let clock = SyntheticClock::at_unix_secs(0);
    let table = Arc::new(Mutex::new(Clocked::begin(
        Position::default(),
        clock.now(),
        Duration::seconds(60),
    )));

    clock.advance_secs(10);
    record(&table, "e2e4", clock.now()); // White has spent 10s
    clock.advance_secs(15);
    record(&table, "e7e5", clock.now()); // Black has spent 15s

    // White is to move with 50s left. Arm the watcher and walk away.
    let _handle = arm(&clock, &table);
    clock.advance_secs(3600);

    assert_that!(clock.fired_callbacks(), eq(1));
    assert_that!(
        table.lock().unwrap().mode(),
        eq(Mode::Played(Ending::Flagged { winner: Color::Black }))
    );
}

#[test]
fn a_move_in_time_cancels_the_claim_and_rearms() {
    let clock = SyntheticClock::at_unix_secs(0);
    let table = Arc::new(Mutex::new(Clocked::begin(
        Position::default(),
        clock.now(),
        Duration::seconds(60),
    )));

    let handle = arm(&clock, &table); // watching White
    clock.advance_secs(30);
    record(&table, "e2e4", clock.now()); // in time, with 30s to spare
    clock.cancel(handle);
    let _handle = arm(&clock, &table); // now watching Black

    clock.advance_secs(3600); // Black never answers

    assert_that!(clock.fired_callbacks(), eq(1));
    assert_that!(
        table.lock().unwrap().mode(),
        eq(Mode::Played(Ending::Flagged { winner: Color::White }))
    );
}
