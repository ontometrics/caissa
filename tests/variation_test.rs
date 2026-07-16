//! PGN variations, read at last: the lexer emits parens as structure,
//! and the parser's one recursion turns nested `(...)` into a Study.

use caissa::pgn::import_study;
use caissa::notation::*;
use caissa::{Game, Piece, Rejected, Role, import};
use googletest::prelude::*;

/// A `(...)` is an alternative to the move it follows; each becomes a
/// line in the study, sharing its prefix with the mainline.
mod reading_variations {
    use super::*;

    #[test]
    fn a_variation_becomes_a_second_line() {
        let study = import_study("1. e4 e5 (1... c5) 2. Nf3 *").unwrap();
        let lines: Vec<Game> = study.lines().collect();

        assert_that!(lines.len(), eq(2));
        assert_that!(lines[0].plies(), eq(3)); // e4 e5 Nf3 — the mainline
        assert_that!(lines[1].plies(), eq(2)); // e4 c5 — the Sicilian aside
        assert_that!(lines[1].position().at(c5), some(eq(Piece::black(Role::Pawn))));
    }

    #[test]
    fn the_mainline_stays_the_mainline() {
        let study = import_study("1. e4 e5 (1... c5) 2. Nf3 *").unwrap();

        assert_that!(
            study.mainline().position(),
            eq(import("1. e4 e5 2. Nf3 *").unwrap().position())
        );
    }

    #[test]
    fn consecutive_variations_are_siblings() {
        // Both (1... c5) and (1... e6) are alternatives to 1... e5.
        let study = import_study("1. e4 e5 (1... c5) (1... e6) 2. Nf3 *").unwrap();
        let lines: Vec<Game> = study.lines().collect();

        assert_that!(lines.len(), eq(3));
        assert_that!(lines[1].position().at(c5), some(eq(Piece::black(Role::Pawn))));
        assert_that!(lines[2].position().at(e6), some(eq(Piece::black(Role::Pawn))));
    }

    #[test]
    fn nested_variations_branch_deeper() {
        // A variation off a move inside a variation.
        let study = import_study("1. e4 e5 (1... c5 2. Nf3 (2. c3) d6) *").unwrap();
        let lines: Vec<Game> = study.lines().collect();

        assert_that!(lines.len(), eq(3));
        assert_that!(lines[0].plies(), eq(2)); // e4 e5
        assert_that!(lines[1].plies(), eq(4)); // e4 c5 Nf3 d6
        assert_that!(lines[2].plies(), eq(3)); // e4 c5 c3
        // The sub-variation shares the Sicilian prefix, not the mainline's.
        assert_that!(lines[2].position().at(c3), some(eq(Piece::white(Role::Pawn))));
        assert_that!(lines[2].position().at(c5), some(eq(Piece::black(Role::Pawn))));
    }

    #[test]
    fn what_the_flat_import_rejects_the_study_import_reads() {
        let text = "1. e4 e5 (1... c5) 2. Nf3 *";

        assert_that!(
            import(text),
            err(eq(&Rejected::Unparseable(
                "variations (...) are not supported".to_string()
            )))
        );
        assert_that!(import_study(text).unwrap().lines().count(), eq(2));
    }
}

/// The parser refuses malformed nesting with the reason as data, and an
/// illegal move inside a variation surfaces the reducer's own verdict.
mod rejections {
    use super::*;

    #[test]
    fn an_unclosed_variation_is_rejected() {
        assert_that!(
            import_study("1. e4 e5 (1... c5"),
            err(eq(&Rejected::Unparseable("unclosed variation".to_string())))
        );
    }

    #[test]
    fn an_unmatched_close_is_rejected() {
        assert_that!(
            import_study("1. e4 ) e5"),
            err(eq(&Rejected::Unparseable("unmatched )".to_string())))
        );
    }

    #[test]
    fn a_variation_before_any_move_is_rejected() {
        assert_that!(
            import_study("(1. d4) 1. e4 *"),
            err(eq(&Rejected::Unparseable(
                "a variation before any move".to_string()
            )))
        );
    }

    #[test]
    fn a_result_inside_a_variation_is_rejected() {
        assert_that!(
            import_study("1. e4 (1. d4 1-0) e5 *"),
            err(eq(&Rejected::Unparseable(
                "a result inside a variation".to_string()
            )))
        );
    }

    #[test]
    fn an_illegal_move_in_a_variation_surfaces_the_reducers_verdict() {
        // "1... e4" is well-formed SAN no black pawn can satisfy.
        let result = import_study("1. e4 e5 (1... e4) *");

        assert_that!(result.is_err(), eq(true));
    }

    #[test]
    fn a_result_that_contradicts_the_mainline_is_rejected() {
        // Fool's Mate with a harmless aside is still a Black win.
        let result = import_study("1. f3 e5 (1... e6) 2. g4 Qh4# 1-0");

        assert_that!(result.is_err(), eq(true));
    }
}
