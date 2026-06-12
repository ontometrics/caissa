use caissa::{Clocked, Color, Ending, Mode, Position, Rejected};
use googletest::prelude::*;

const BUDGET: u32 = 60;

/// Begin at 0; White thinks 10, Black 15, White 20. Black to move.
fn midgame() -> Clocked<u32, u32> {
    Clocked::begin(Position::default(), 0u32, BUDGET)
        .record("e2e4", 10)
        .and_then(|c| c.record("e7e5", 25))
        .and_then(|c| c.record("g1f3", 45))
        .unwrap()
}

mod spending {
    use super::*;

    #[test]
    fn spent_time_is_a_fold_over_each_players_frames() {
        let clocked = midgame();

        assert_that!(clocked.spent(Color::White, 45), eq(30)); // 10 + 20
        assert_that!(clocked.spent(Color::Black, 45), eq(15));
    }

    #[test]
    fn the_clock_ticks_only_against_the_player_to_move() {
        let clocked = midgame();

        assert_that!(clocked.spent(Color::Black, 65), eq(35)); // 15 + 20 ticking
        assert_that!(clocked.spent(Color::White, 65), eq(30)); // unchanged
    }

    #[test]
    fn remaining_is_the_number_on_the_clock_face() {
        let clocked = midgame();

        assert_that!(clocked.remaining(Color::White, 45), eq(30)); // 60 - 30
        assert_that!(clocked.remaining(Color::Black, 65), eq(25)); // 60 - 35, ticking
        assert_that!(clocked.remaining(Color::Black, 110), eq(0)); // flag is down
    }
}

mod flagging {
    use super::*;

    #[test]
    fn a_move_stamped_past_the_budget_is_out_of_time() {
        let result = Clocked::begin(Position::default(), 0u32, BUDGET).record("e2e4", 70);

        assert_that!(result, err(eq(&Rejected::OutOfTime)));
    }

    #[test]
    fn a_claim_while_the_flag_is_up_is_rejected() {
        let result = midgame().claim_flag(50);

        assert_that!(result, err(eq(&Rejected::StillOnTime)));
    }

    #[test]
    fn an_expired_budget_loses_on_time_when_claimed() {
        // Black stops moving; by 110 they have spent 15 + 65 > 60.
        let flagged = midgame().claim_flag(110).unwrap();

        assert_that!(
            flagged.mode(),
            eq(Mode::Played(Ending::Flagged { winner: Color::White }))
        );
        assert_that!(flagged.ended(), some(eq(110)));
    }

    #[test]
    fn nothing_records_after_a_flag_falls() {
        let flagged = midgame().claim_flag(110).unwrap();

        let result = flagged.record("b8c6", 120);

        assert_that!(
            result,
            err(eq(&Rejected::GameOver {
                ending: Ending::Flagged { winner: Color::White }
            }))
        );
    }

    #[test]
    fn an_unclaimed_flag_keeps_the_game_playing() {
        // Over the board, an unnoticed flag is no flag: the obligation is
        // data, the claim is the event, and nothing happens by itself.
        let clocked = midgame();

        assert_that!(clocked.mode(), eq(Mode::Playing));
    }
}
