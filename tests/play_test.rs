use caissa::play::{Random, between};
use caissa::{Game, Mode, Position};
use googletest::prelude::*;

/// A self-play game is a reproducible value: the seed is the parameter.
mod determinism {
    use super::*;

    #[test]
    fn the_same_seeds_play_the_same_game() {
        let first = between(&Random::seeded(1), &Random::seeded(2), Position::default());
        let again = between(&Random::seeded(1), &Random::seeded(2), Position::default());

        assert_that!(&first, eq(&again));
    }

    #[test]
    fn different_seeds_diverge() {
        let one = between(&Random::seeded(1), &Random::seeded(2), Position::default());
        let other = between(&Random::seeded(7), &Random::seeded(9), Position::default());

        assert_that!(one.log() == other.log(), eq(false));
    }
}

/// Headless play always finishes, and the game it yields is a real game.
mod a_finished_game {
    use super::*;

    #[test]
    fn a_random_game_always_ends() {
        // The 75-move and fivefold draws cap every line, so the loop halts.
        let game = between(
            &Random::seeded(42),
            &Random::seeded(43),
            Position::default(),
        );

        assert_that!(game.mode() == Mode::Playing, eq(false));
    }

    #[test]
    fn the_result_is_a_valid_game() {
        let game = between(&Random::seeded(5), &Random::seeded(6), Position::default());

        // Replaying its log reproduces it — self-play produces real games.
        assert_that!(
            Game::replay(Position::default(), game.log().to_vec()),
            ok(eq(&game.position()))
        );
    }

    #[test]
    fn a_played_game_scores_a_real_result() {
        let game = between(&Random::seeded(8), &Random::seeded(13), Position::default());

        // Not unfinished — the score ends with an outcome, never "*".
        assert_that!(game.score().ends_with('*'), eq(false));
    }
}
