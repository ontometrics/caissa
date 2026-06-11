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

## Status

Early. Full piece movement, captures, promotion, check/checkmate/
stalemate, time controls. Not yet: castling, en passant, draw rules
(repetition, fifty-move, insufficient material).

## License

MIT OR Apache-2.0
