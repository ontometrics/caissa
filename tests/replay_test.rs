use caissa::notation::*;
use caissa::{Color, Ending, Frame, Piece, Position, Rejected, Role, Timeline};
use googletest::prelude::*;

/// Stamps are plain integers in tests — think milliseconds on a blitz clock.
fn blitz() -> Timeline<u32> {
    Timeline::begin(Position::default(), 0)
        .record("e2e4", 300)
        .and_then(|t| t.record("e7e5", 1100))
        .and_then(|t| t.record("g1f3", 1300))
        .unwrap()
}

#[test]
fn frames_carry_each_moves_think_time() {
    let think_times: Vec<u32> = blitz().frames().map(|frame| frame.think_time).collect();

    assert_that!(think_times, elements_are![eq(&300), eq(&800), eq(&200)]);
}

#[test]
fn frames_replay_the_positions_in_order() {
    let timeline = blitz();

    let last: Frame<u32> = timeline.frames().last().unwrap();

    assert_that!(last.position, eq(timeline.game().position()));
    assert_that!(
        last.position.at(f3),
        some(eq(Piece { color: Color::White, role: Role::Knight }))
    );
}

#[test]
fn time_only_moves_forward() {
    let result = blitz().record("b8c6", 900);

    assert_that!(result, err(eq(&Rejected::OutOfOrder)));
}

#[test]
fn the_clock_never_excuses_an_illegal_move() {
    let result = Timeline::begin(Position::default(), 0u32).record("e7e5", 250);

    assert_that!(
        result,
        err(eq(&Rejected::NotYourTurn {
            piece: Piece { color: Color::Black, role: Role::Pawn }
        }))
    );
}

#[test]
fn an_ongoing_game_has_no_end() {
    let timeline = blitz();

    assert_that!(timeline.started(), eq(0));
    assert_that!(timeline.ended(), none());
}

#[test]
fn the_interval_closes_with_the_move_that_ends_the_game() {
    let timeline = Timeline::begin(Position::default(), 0u32)
        .record("f2f3", 5)
        .and_then(|t| t.record("e7e5", 9))
        .and_then(|t| t.record("g2g4", 14))
        .and_then(|t| t.record("d8h4", 21))
        .unwrap();

    assert_that!(timeline.ended(), some(eq(21)));
}

#[test]
fn nothing_records_after_the_end() {
    let finished = Timeline::begin(Position::default(), 0u32)
        .record("f2f3", 5)
        .and_then(|t| t.record("e7e5", 9))
        .and_then(|t| t.record("g2g4", 14))
        .and_then(|t| t.record("d8h4", 21))
        .unwrap();

    let result = finished.record("a2a3", 30);

    assert_that!(
        result,
        err(eq(&Rejected::GameOver {
            ending: Ending::Checkmate { winner: Color::Black }
        }))
    );
}

#[test]
fn a_rejected_record_leaves_the_timeline_untouched() {
    let timeline = blitz();

    let _ = timeline.record("b8c6", 900);

    assert_that!(timeline.game().plies(), eq(3));
}
