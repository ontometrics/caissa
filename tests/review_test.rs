use caissa::classics::{fools_mate, ruy_lopez};
use caissa::play::{evaluate, material};
use caissa::review::annotate;
use caissa::{Color, Position};
use googletest::prelude::*;

/// A one-ply tactical eval — enough to see a mate in one, fast enough to
/// annotate a short game instantly.
fn tactical(position: Position) -> i32 {
    evaluate(position, 1, &material)
}

/// The shift series has one entry per ply, movers alternating.
mod the_shift_series {
    use super::*;

    #[test]
    fn one_assessment_per_ply() {
        let series = annotate(&fools_mate(), &tactical);

        assert_that!(series.len(), eq(4));
        assert_that!(series[0].ply, eq(1));
        assert_that!(series[3].ply, eq(4));
    }

    #[test]
    fn the_movers_alternate_white_first() {
        let series = annotate(&fools_mate(), &tactical);

        assert_that!(series[0].mover, eq(Color::White));
        assert_that!(series[1].mover, eq(Color::Black));
        assert_that!(series[2].mover, eq(Color::White));
        assert_that!(series[3].mover, eq(Color::Black));
    }
}

/// The worst swing is the turning point — who fumbled, and on what move.
mod the_turning_point {
    use super::*;

    #[test]
    fn it_finds_the_fumble_that_allowed_mate() {
        // Fool's Mate: White's 2. g4?? (ply 3) lets Black play Qh4#.
        let series = annotate(&fools_mate(), &tactical);
        let worst = series.iter().min_by_key(|a| a.swing).unwrap();

        assert_that!(worst.ply, eq(3));
        assert_that!(worst.mover, eq(Color::White));
        assert_that!(worst.swing < -100_000, eq(true)); // a mate-scale loss
    }

    #[test]
    fn a_clean_opening_has_no_catastrophe() {
        // The Ruy Lopez: principled moves, no blunder near mate scale.
        let series = annotate(&ruy_lopez(), &tactical);
        let worst = series.iter().map(|a| a.swing).min().unwrap();

        assert_that!(worst > -100_000, eq(true));
    }
}
