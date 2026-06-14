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
        let board_says = match game.mode() {
            Mode::Played(Ending::Checkmate { winner: Color::White }) => Some("1-0"),
            Mode::Played(Ending::Checkmate { winner: Color::Black }) => Some("0-1"),
            Mode::Played(Ending::Stalemate) | Mode::Played(Ending::Draw(_)) => Some("1/2-1/2"),
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
    let board_says = match game.mode() {
        Mode::Played(Ending::Checkmate { winner: Color::White }) => Some("1-0"),
        Mode::Played(Ending::Checkmate { winner: Color::Black }) => Some("0-1"),
        Mode::Played(Ending::Stalemate) | Mode::Played(Ending::Draw(_)) => Some("1/2-1/2"),
        _ => None,
    };
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
