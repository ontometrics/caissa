use std::collections::BTreeMap;

use caissa::classics::{fools_mate, opera_game, ruy_lopez};
use caissa::{Rejected, import, pgn};
use googletest::prelude::*;

fn tags(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|&(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

/// Export then import is the identity: the game, the tags, and the
/// result all survive the round trip.
mod round_trips {
    use super::*;

    #[test]
    fn the_opera_game_survives_export_and_reimport() {
        let game = opera_game();
        let exported = pgn::export(
            &game,
            &tags(&[("Event", "Paris Opera"), ("White", "Paul Morphy")]),
        )
        .unwrap();

        let again = import(&exported).unwrap();
        assert_that!(again.plies(), eq(game.plies()));
        assert_that!(again.position(), eq(game.position()));

        let parsed = pgn::parse(&exported).unwrap();
        assert_that!(
            parsed.tags.get("White").map(String::as_str),
            some(eq("Paul Morphy"))
        );
        assert_that!(parsed.result.as_deref(), some(eq("1-0")));
    }

    #[test]
    fn movetext_wraps_at_eighty_columns() {
        let exported = pgn::export(&opera_game(), &tags(&[])).unwrap();

        for line in exported.lines() {
            assert_that!(line.len() <= 80, eq(true));
        }
    }

    #[test]
    fn the_roster_tags_lead_in_canonical_order() {
        let exported = pgn::export(
            &opera_game(),
            &tags(&[("White", "Paul Morphy"), ("Event", "Paris Opera"), ("Zoo", "extra")]),
        )
        .unwrap();

        let lines: Vec<&str> = exported.lines().collect();
        assert_that!(lines[0].starts_with("[Event"), eq(true));
        assert_that!(lines[1].starts_with("[White"), eq(true));
        assert_that!(lines[2].starts_with("[Result"), eq(true));
        assert_that!(lines[3].starts_with("[Zoo"), eq(true));
    }
}

/// The Result field is negotiated the same way import verifies it: the
/// board where it knows, the tag where it cannot, never a contradiction.
mod honesty {
    use super::*;

    #[test]
    fn the_board_supplies_the_result_it_knows() {
        let exported = pgn::export(&fools_mate(), &tags(&[])).unwrap();

        assert_that!(exported.contains("[Result \"0-1\"]"), eq(true));
        assert_that!(exported.trim_end().ends_with("0-1"), eq(true));
    }

    #[test]
    fn a_resignation_the_board_cannot_see_is_taken_from_the_tag() {
        let exported = pgn::export(&ruy_lopez(), &tags(&[("Result", "1-0")])).unwrap();

        assert_that!(exported.contains("[Result \"1-0\"]"), eq(true));
        assert_that!(exported.trim_end().ends_with("3. Bb5 1-0"), eq(true));

        let again = import(&exported).unwrap();
        assert_that!(again.plies(), eq(5));
    }

    #[test]
    fn an_unfinished_untagged_game_exports_a_star() {
        let exported = pgn::export(&ruy_lopez(), &tags(&[])).unwrap();

        assert_that!(exported.contains("[Result \"*\"]"), eq(true));
        assert_that!(exported.trim_end().ends_with('*'), eq(true));
    }

    #[test]
    fn a_tag_that_contradicts_the_board_is_rejected() {
        let result = pgn::export(&fools_mate(), &tags(&[("Result", "1-0")]));

        assert_that!(
            result,
            err(eq(&Rejected::Unparseable(
                "result 1-0 contradicts the board, which says 0-1".to_string()
            )))
        );
    }
}
