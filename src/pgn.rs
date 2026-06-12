//! PGN import: tag pairs, movetext, fold.
//!
//! A PGN is a game written down — so importing one is the fold the crate
//! is built on: parse the movetext into SAN tokens, then
//! `sans.try_fold(Game::new(), Game::apply)`. Comments (`{...}`, `;` to
//! end of line), move numbers, and NAGs (`$n`) are skipped; variations
//! (`(...)`) are rejected loudly rather than mis-parsed. The result
//! marker, when present, is checked against what the board actually says.

use std::collections::BTreeMap;

use crate::game::Game;
use crate::piece::Color;
use crate::reduce::{Ending, Mode, Rejected};

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

    let stripped = strip_comments(&movetext)?;
    let mut sans = Vec::new();
    let mut result = None;
    for token in stripped.split_whitespace() {
        match token {
            "1-0" | "0-1" | "1/2-1/2" | "*" => {
                result = Some(token.to_string());
                break;
            }
            _ if token.starts_with('$') => continue,
            _ => {}
        }
        let san = strip_move_number(token);
        if !san.is_empty() {
            sans.push(san.to_string());
        }
    }
    Ok(Pgn { tags, sans, result })
}

/// Import a PGN as a played [`Game`]: the movetext folded over
/// [`Game::apply`]. A result marker that contradicts what the board says
/// (a checkmate for the other side, say) is rejected; markers the board
/// cannot verify — resignations, agreed draws, flag falls — are accepted
/// as written.
pub fn import(text: &str) -> Result<Game, Rejected> {
    let pgn = parse(text)?;
    let game = pgn
        .sans
        .iter()
        .try_fold(Game::new(), |game, san| game.apply(san.as_str()))?;
    if let Some(written) = pgn.result.as_deref() {
        let board_says = match game.mode() {
            Mode::Played(Ending::Checkmate { winner: Color::White }) => Some("1-0"),
            Mode::Played(Ending::Checkmate { winner: Color::Black }) => Some("0-1"),
            Mode::Played(Ending::Stalemate) => Some("1/2-1/2"),
            _ => None,
        };
        if let Some(expected) = board_says
            && written != expected
        {
            return Err(Rejected::Unparseable(format!(
                "result {written} contradicts the board, which says {expected}"
            )));
        }
    }
    Ok(game)
}

fn tag_pair(line: &str) -> Option<(String, String)> {
    let inner = line.strip_prefix('[')?.strip_suffix(']')?;
    let (key, rest) = inner.split_once(' ')?;
    let value = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
    Some((key.to_string(), value.to_string()))
}

fn strip_comments(text: &str) -> Result<String, Rejected> {
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
                return Err(Rejected::Unparseable(
                    "variations (...) are not supported".to_string(),
                ));
            }
            _ => out.push(c),
        }
    }
    Ok(out)
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
