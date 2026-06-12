# caissa

A functional chess library: positions are values, moves are pure
transitions, a game is a fold.

```rust
use caissa::Position;

let position = ["e2e4", "e7e5", "g1f3"]
    .into_iter()
    .try_fold(Position::default(), Position::play)?;
```

Or in operator notation (`->` is not overloadable in Rust; `>>` is the
arrow-shaped operator that is):

```rust
use caissa::notation::*;

let position = (Position::default() + (e2 >> e4) + (e7 >> e5) + (g1 >> f3))?;
```

## The idea

An action carries no more than the player's intent — a from-square and a
to-square — because the position already knows everything else: what
stands on the square, whether the target is a capture. The one exception
chess forces is promotion, which gets its own action variant (`h7h8q`,
or `h7 >> h8 >> queen`).

`Position` is a plain `Copy` value, so persistence is free: every
transition yields a new position and the old one stays valid. History,
search trees, undo, and variations are all "keep the old value".

Errors are data. One `Rejected` enum covers everything from unparseable
input to `IntoCheck { king }` — a single rule ("the resulting position may
not leave your king attacked") that covers pins, moving into check, and
ignoring check.

## The layers

Each layer adds one concern, built from the one below:

| Layer | Concern | Highlights |
|---|---|---|
| `reduce` | legality | the pure transition `Position × Action → Result<Position, Rejected>` |
| `Game` | memory | memoized fold (`history`), jump notation `game[n]`, `game[Terminus - 1]`, `Mode::{Playing, Played}` |
| `Timeline<T>` | chronology | timestamps as data, realtime replay via `frames()`, Snodgrass interval (`ended() == None` while playing) |
| `Clocked<T, D>` | obligation | per-player budgets, `spent` derived as a fold, `claim_flag` — an unclaimed flag keeps the game playing |

The timestamp type is generic: tests use plain integers, a real recorder
uses `Instant` or `DateTime<Utc>`. The core never reads a clock — time
enters as data, and the only effect in a speedchess replay is the `sleep`
between frames:

```rust
for frame in timeline.frames() {
    std::thread::sleep(frame.think_time);   // exactly as long as the player thought
    render(frame.position);
}
```

## Seeing the board

`Position` implements `Display`:

```text
8 ♜ ♞ ♝ ♛ ♚ ♝ ♞ ♜
7 ♟ ♟ ♟ ♟ ♟ ♟ ♟ ♟
6 · · · · · · · ·
5 · · · · · · · ·
4 · · · · ♙ · · ·
3 · · · · · · · ·
2 ♙ ♙ ♙ ♙ · ♙ ♙ ♙
1 ♖ ♘ ♗ ♕ ♔ ♗ ♘ ♖
  a b c d e f g h
```

Castling needs no notation of its own: it is the king's two-square move
(`e1g1` / `e1c1`, UCI-style) — a king can never legally travel two squares
any other way, so intent stays from–to. `O-O` is import-time sugar. En
passant is likewise just the diagonal pawn move onto the skipped square.
Both cost `Position` its first memory — castling rights and the en-passant
square ride along in the value, exactly the fields FEN has always carried.

## Classic games

SAN is resolved, not interpreted: `"Nf3"` parses into a description, and
resolution filters the legal actions down to the unique match — so SAN
inherits every rule the reducer enforces, disambiguation included. PGN
import is then just the fold the crate is built on:

```rust
let game = caissa::import(OPERA_GAME_PGN)?;
assert_that!(
    game.mode(),
    eq(Mode::Played(Ending::Checkmate { winner: Color::White }))
);
println!("{}", game[Terminus]);
```

`game.apply("e4")`, `game.apply("Nbd2")`, and `game.apply("O-O")` all
work directly; UCI and SAN mix freely at every call site.

And games write themselves back out, newspaper-style — `game.score()`
(also its `Display`) emits minimally-disambiguated SAN with check and
mate suffixes supplied by the reducer, and `game.figurines()` gives the
publication figurine form:

```text
1. e4 e5 2. Nf3 d6 … 16. Qb8+ Nxb8 17. Rd8# 1-0
1. e4 e5 2. ♘f3 d6 … 16. ♕b8+ ♞xb8 17. ♖d8# 1-0
```

Emission is resolution's inverse, and the tests hold it to that:
`import(game.score())` reproduces the game.

Chess is now rules-complete: every way a game can legally move or end is
in the reducer and the layers above it.

## Status

- [x] Pure reducer — from–to actions, one error type, errors as data — `v0.1.0`
- [x] Full piece movement, captures, promotion — `v0.1.0`
- [x] Check, checkmate, stalemate — one rule: never leave your king attacked — `v0.1.0`
- [x] Castling (the king's two-square move) and en passant — `v0.1.0`
- [x] `Mode::{Playing, Played}` — O(1) game-over gating, no move checking after the end — `v0.1.0`
- [x] Memoized history with jump notation: `game[n]`, `game[Terminus - 1]` — `v0.1.0`
- [x] Timestamped `Timeline`, realtime replay, Snodgrass interval (`ended() == None` while playing) — `v0.1.0`
- [x] `Clocked` speedchess: per-player budgets, derived clocks, flag claims — `v0.1.0`
- [x] Operator notation: `e2 >> e4`, `position + action` — `v0.1.0`
- [x] Board `Display` (Unicode, rank 8 up) — `v0.1.0`
- [x] The interpreter: actions expand from prototypes into `Edit`s, applied by a total evaluator — `v0.2.0`
- [x] SAN resolved against `legal_actions()`; UCI and SAN mix freely — `v0.3.0`
- [x] PGN import — the Opera Game and the Immortal Game fold to mate — `v0.3.0`
- [x] Score emission, letters and figurines; `import(game.score())` round-trips — `v0.4.0`
- [x] `classics` module — famous games and openings as ready-made fixtures, plus `Piece::white`/`Piece::black` — `v0.5.0`
- [x] Draw rules — insufficient material on the board; repetition and move-counts derived from the history; threefold/fifty claimed (`claim_draw`), fivefold/seventy-five automatic — `v0.6.0`
- [ ] Full PGN export with tag pairs
- [ ] FEN import/export (start positions for `Game::from_position`)
- [ ] Variations — parsed and represented, not just rejected
- [ ] Engine fast path: ungated movegen for search workloads

## License

MIT OR Apache-2.0
