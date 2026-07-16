//! PGN import: tag pairs, movetext, fold.
//!
//! A PGN is a game written down — so importing one is the fold the crate
//! is built on: parse the movetext into SAN tokens, then
//! `sans.try_fold(Game::new(), Game::apply)`. Comments (`{...}`, `;` to
//! end of line), move numbers, and NAGs (`$n`) are skipped. Variations
//! (`(...)`) are rejected by the flat path — a `Game` is a line — and
//! read by [`import_study`], which parses their nesting into a
//! [`Study`]. The result marker, when present, is checked against what
//! the board actually says.

use std::collections::BTreeMap;

use crate::game::Game;
use crate::piece::Color;
use crate::reduce::{Ending, Mode, Rejected};
use crate::study::Study;

/// A parsed PGN: its tag pairs, the SAN tokens of the movetext, and the
/// result marker if one was written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pgn {
    pub tags: BTreeMap<String, String>,
    pub sans: Vec<String>,
    pub result: Option<String>,
}

/// Parse PGN text into tags and SAN tokens, without playing anything.
pub fn parse(text: &str) -> Result<Pgn, Rejected> {
    let (tags, movetext) = split_tags(text)?;
    let mut sans = Vec::new();
    let mut result = None;
    for token in lex(&movetext) {
        match token {
            Token::San(san) => sans.push(san),
            Token::Result(marker) => {
                result = Some(marker);
                break;
            }
            // A Game is a line; the flat path cannot represent branches.
            // import_study is what (...) means.
            Token::Open | Token::Close => {
                return Err(Rejected::Unparseable(
                    "variations (...) are not supported".to_string(),
                ));
            }
        }
    }
    Ok(Pgn { tags, sans, result })
}

/// The tag section from the movetext: tag lines lead, everything after
/// the first non-tag line is movetext.
fn split_tags(text: &str) -> Result<(BTreeMap<String, String>, String), Rejected> {
    let mut tags = BTreeMap::new();
    let mut movetext = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if movetext.is_empty() {
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('[') {
                let (key, value) =
                    tag_pair(trimmed).ok_or_else(|| Rejected::Unparseable(trimmed.to_string()))?;
                tags.insert(key, value);
                continue;
            }
        }
        movetext.push_str(line);
        movetext.push('\n');
    }
    Ok((tags, movetext))
}

/// One token of movetext. The token enum is the signature of a lexer
/// that has earned its name: the state machine handles the regular
/// sublanguage (comments, move numbers, NAGs), and the parens — the
/// characters that make the language context-free — pass upward as
/// structure for the parser's one recursion.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Token {
    San(String),
    Open,
    Close,
    Result(String),
}

/// The lexer, named at last: raw movetext to tokens. Text → tokens
/// (lex) → tree (parse) → [`Study`] (fold) — the same interpreter shape
/// as `Action → Edits → Position`, one altitude over text.
fn lex(movetext: &str) -> Vec<Token> {
    strip_comments(movetext)
        .split_whitespace()
        .filter_map(|word| match word {
            "(" => Some(Token::Open),
            ")" => Some(Token::Close),
            "1-0" | "0-1" | "1/2-1/2" | "*" => Some(Token::Result(word.to_string())),
            _ if word.starts_with('$') => None,
            _ => {
                let san = strip_move_number(word);
                (!san.is_empty()).then(|| Token::San(san.to_string()))
            }
        })
        .collect()
}

/// Import a PGN as a played [`Game`]: the movetext folded over
/// [`Game::apply`]. A result marker that contradicts what the board says
/// (a checkmate for the other side, say) is rejected; markers the board
/// cannot verify — resignations, agreed draws, flag falls — are accepted
/// as written. Of a multi-game file, only the first game is read
/// (parsing stops at the first result marker); use [`import_all`] or
/// [`games`] for a whole database.
pub fn import(text: &str) -> Result<Game, Rejected> {
    let pgn = parse(text)?;
    let game = pgn
        .sans
        .iter()
        .try_fold(Game::new(), |game, san| game.apply(san.as_str()))?;
    if let Some(written) = pgn.result.as_deref() {
        verify_result(game.mode(), written)?;
    }
    Ok(game)
}

/// Import a PGN *with variations* as a [`Study`]. A `(...)` follows a
/// move and holds alternatives to it, nested to any depth, so each
/// variation's line is the prefix before the varied move plus its own
/// moves — every line folded into a `Game` and grafted with
/// [`Study::with`], mainline first. The flat [`import`] still rejects
/// variations, because a `Game` is a line; a `Study` is what `(...)`
/// means. The result marker, when present, is checked against the
/// mainline's board exactly as [`import`] checks a game's.
pub fn import_study(text: &str) -> Result<Study, Rejected> {
    let (_tags, movetext) = split_tags(text)?;
    let tokens = lex(&movetext);
    let mut cursor = 0;
    let mut result = None;
    let lines = sequence(&tokens, &mut cursor, Vec::new(), false, &mut result)?;

    let mut study = Study::new();
    let mut mainline_mode = None;
    for line in &lines {
        let game = line
            .iter()
            .try_fold(Game::new(), |game, san| game.apply(san.as_str()))?;
        if mainline_mode.is_none() {
            mainline_mode = Some(game.mode());
        }
        study = study.with(game);
    }
    if let (Some(mode), Some(written)) = (mainline_mode, result.as_deref()) {
        verify_result(mode, written)?;
    }
    Ok(study)
}

/// The parser's one recursion: read a sequence of moves, and at each
/// `(` read alternatives to the move just played (their prefix = the
/// current line minus that move), returning at `)`. A sequence returns
/// its own complete line *first*, then its variations' lines in textual
/// order — so grafting in return order keeps the mainline child 0 at
/// every node.
fn sequence(
    tokens: &[Token],
    cursor: &mut usize,
    prefix: Vec<String>,
    nested: bool,
    result: &mut Option<String>,
) -> Result<Vec<Vec<String>>, Rejected> {
    let mut line = prefix;
    let mut variations = Vec::new();
    loop {
        let Some(token) = tokens.get(*cursor) else {
            if nested {
                return Err(Rejected::Unparseable("unclosed variation".to_string()));
            }
            break;
        };
        *cursor += 1;
        match token {
            Token::San(san) => line.push(san.clone()),
            Token::Open => {
                let Some((_, base)) = line.split_last() else {
                    return Err(Rejected::Unparseable(
                        "a variation before any move".to_string(),
                    ));
                };
                variations.extend(sequence(tokens, cursor, base.to_vec(), true, result)?);
            }
            Token::Close => {
                if !nested {
                    return Err(Rejected::Unparseable("unmatched )".to_string()));
                }
                break;
            }
            Token::Result(marker) => {
                if nested {
                    return Err(Rejected::Unparseable(
                        "a result inside a variation".to_string(),
                    ));
                }
                *result = Some(marker.clone());
                break;
            }
        }
    }
    let mut lines = vec![line];
    lines.extend(variations);
    Ok(lines)
}

/// What the board attests the result to be, when it can.
fn board_result(mode: Mode) -> Option<&'static str> {
    match mode {
        Mode::Played(Ending::Checkmate { winner: Color::White }) => Some("1-0"),
        Mode::Played(Ending::Checkmate { winner: Color::Black }) => Some("0-1"),
        Mode::Played(Ending::Stalemate) | Mode::Played(Ending::Draw(_)) => Some("1/2-1/2"),
        _ => None,
    }
}

fn verify_result(mode: Mode, written: &str) -> Result<(), Rejected> {
    if let Some(expected) = board_result(mode)
        && written != expected
    {
        return Err(Rejected::Unparseable(format!(
            "result {written} contradicts the board, which says {expected}"
        )));
    }
    Ok(())
}

/// Split a PGN database into its games — contiguous slices, no copying.
/// A new game begins at a tag line that follows movetext, the standard
/// boundary (every database game carries its seven-tag roster). Pair it
/// with [`import`] for the lenient path over a corpus —
/// `games(db).into_iter().map(import)` keeps each game's result as data,
/// so one bad game does not sink the rest.
pub fn games(text: &str) -> Vec<&str> {
    let mut starts = vec![0];
    let mut seen_moves = false;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        let is_tag = trimmed.starts_with('[');
        if is_tag && seen_moves {
            starts.push(offset); // a tag after movetext starts the next game
            seen_moves = false;
        } else if !is_tag && !trimmed.is_empty() {
            seen_moves = true;
        }
        offset += line.len();
    }
    starts
        .iter()
        .enumerate()
        .filter_map(|(i, &start)| {
            let end = starts.get(i + 1).copied().unwrap_or(text.len());
            let chunk = text[start..end].trim();
            (!chunk.is_empty()).then_some(chunk)
        })
        .collect()
}

/// Import every game in a database — the corpus loader the dictionary,
/// the annotator, and the repertoire all want. Strict: the first
/// unparseable game fails the batch (the `Rejected` says which). For
/// lenient loading, map [`import`] over [`games`] and keep the per-game
/// `Result`s.
pub fn import_all(text: &str) -> Result<Vec<Game>, Rejected> {
    games(text).into_iter().map(import).collect()
}

/// Export a game as PGN: tag pairs (seven-tag-roster order first, the
/// rest alphabetically), a blank line, then movetext wrapped at 80
/// columns. The Result comes from the board where it knows
/// (mate, stalemate, draws) and from the declared tag where it cannot
/// (resignations, agreements); a declared result that contradicts the
/// board is rejected, mirroring [`import`].
pub fn export(game: &Game, tags: &BTreeMap<String, String>) -> Result<String, Rejected> {
    let board_says = board_result(game.mode());
    let declared = tags.get("Result").map(String::as_str);
    let result = match (board_says, declared) {
        (Some(board), Some(tag)) if board != tag => {
            return Err(Rejected::Unparseable(format!(
                "result {tag} contradicts the board, which says {board}"
            )));
        }
        (Some(board), _) => board,
        (None, Some(tag)) => tag,
        (None, None) => "*",
    };

    const ROSTER: [&str; 7] = ["Event", "Site", "Date", "Round", "White", "Black", "Result"];
    let mut out = String::new();
    for key in ROSTER {
        if key == "Result" {
            out.push_str(&format!("[Result \"{result}\"]\n"));
        } else if let Some(value) = tags.get(key) {
            out.push_str(&format!("[{key} \"{value}\"]\n"));
        }
    }
    for (key, value) in tags {
        if !ROSTER.contains(&key.as_str()) {
            out.push_str(&format!("[{key} \"{value}\"]\n"));
        }
    }
    out.push('\n');

    // The score already ends with the board's marker; the movetext keeps
    // the moves and takes the negotiated result instead.
    let score = game.score();
    let moves = score.rsplit_once(' ').map_or("", |(moves, _)| moves);
    for line in wrap(&format!("{moves} {result}"), 80) {
        out.push_str(&line);
        out.push('\n');
    }
    Ok(out)
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn tag_pair(line: &str) -> Option<(String, String)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let (key, rest) = inner.split_once(' ')?;
    let value = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
    Some((key.to_string(), value.to_string()))
}

fn strip_comments(text: &str) -> String {
    let mut out = String::new();
    let mut in_brace = false;
    let mut in_line = false;
    for c in text.chars() {
        match c {
            '\n' => {
                in_line = false;
                out.push(' ');
            }
            _ if in_line => {}
            '{' if !in_brace => in_brace = true,
            '}' if in_brace => in_brace = false,
            _ if in_brace => {}
            ';' => in_line = true,
            '(' | ')' => {
                // Structural tokens for the parser, spaced so they lex
                // cleanly even glued to a move: "(3." or "Nf6)".
                out.push(' ');
                out.push(c);
                out.push(' ');
            }
            _ => out.push(c),
        }
    }
    out
}

/// `"1.e4"` → `"e4"`, `"1."` / `"1..."` → `""`; `"0-0"` is left whole —
/// digits without dots are notation, not numbering.
fn strip_move_number(token: &str) -> &str {
    let digits = token.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return token;
    }
    let rest = &token[digits..];
    let dots = rest.chars().take_while(|&c| c == '.').count();
    if dots == 0 { token } else { &rest[dots..] }
}
