use caissa::classics::ruy_lopez;
use caissa::notation::*;
use caissa::study::Study;
use caissa::{Color, Game, Piece, Role};
use googletest::prelude::*;

fn white(role: Role) -> Piece {
    Piece {
        color: Color::White,
        role,
    }
}

/// A game is a study of one line; an empty study is one trivial line.
mod a_study_of_one_line {
    use super::*;

    #[test]
    fn a_game_becomes_its_mainline() {
        let game = ruy_lopez();
        let study = Study::from(game.clone());

        assert_that!(study.mainline().position(), eq(game.position()));
        assert_that!(study.lines().count(), eq(1));
    }

    #[test]
    fn an_empty_study_is_one_trivial_line() {
        let study = Study::new();

        assert_that!(study.mainline().plies(), eq(0));
        assert_that!(study.lines().count(), eq(1));
        assert_that!(&Study::default(), eq(&study));
    }
}

/// Grafting merges along the shared prefix and branches at the divergence.
mod grafting {
    use super::*;

    fn ruy_with_italian_alternative() -> Study {
        let mainline = ruy_lopez(); // …Nc6 3. Bb5
        let variation = mainline.undo().apply("Bc4").unwrap(); // …Nc6 3. Bc4
        Study::from(mainline).with(variation)
    }

    #[test]
    fn a_variation_yields_two_lines_sharing_their_prefix() {
        let study = ruy_with_italian_alternative();
        let lines: Vec<Game> = study.lines().collect();

        assert_that!(lines.len(), eq(2));
        // The first four plies are shared — identical logs up to the branch.
        assert_that!(&lines[0].log()[..4], eq(&lines[1].log()[..4]));
        // …and they diverge at the fifth: Bb5 versus Bc4.
        assert_that!(lines[0][5].at(b5), some(eq(white(Role::Bishop))));
        assert_that!(lines[1][5].at(c4), some(eq(white(Role::Bishop))));
    }

    #[test]
    fn the_first_line_grafted_stays_the_mainline() {
        let study = ruy_with_italian_alternative();

        assert_that!(study.mainline()[5].at(b5), some(eq(white(Role::Bishop))));
    }

    #[test]
    fn re_grafting_the_same_line_changes_nothing() {
        let game = ruy_lopez();
        let once = Study::from(game.clone());
        let twice = once.clone().with(game);

        assert_that!(&twice, eq(&once));
        assert_that!(twice.lines().count(), eq(1));
    }
}

/// Variations nest: a variation off a variation branches deeper, and the
/// mainline is unmoved.
mod nesting {
    use super::*;

    #[test]
    fn a_sub_variation_branches_deeper() {
        let mainline = ruy_lopez(); // …Nc6 3. Bb5
        let italian = mainline.undo().apply("Bc4").unwrap(); // …3. Bc4
        let italian_classical = italian.apply("Bc5").unwrap(); // …3. Bc4 Bc5
        let two_knights = italian.apply("Nf6").unwrap(); // …3. Bc4 Nf6

        let study = Study::from(mainline)
            .with(italian_classical)
            .with(two_knights);

        let lines: Vec<Game> = study.lines().collect();

        // Three leaves: the Bb5 mainline, and two replies under Bc4.
        assert_that!(lines.len(), eq(3));
        // Mainline untouched by the nested branching.
        assert_that!(study.mainline()[5].at(b5), some(eq(white(Role::Bishop))));
        // The two sub-variations share five plies (through Bc4) then split.
        assert_that!(&lines[1].log()[..5], eq(&lines[2].log()[..5]));
        assert_that!(lines[1].plies(), eq(6));
        assert_that!(lines[2].plies(), eq(6));
    }
}
