# Plan: draw rules (v0.6.0)

The last rules standing between caissa and complete chess. The goal,
stated as the tests that end the work:

```rust
// Knights out and back, twice over — the position recurs a third time.
let game = shuffle_to_threefold();
let drawn = game.claim_draw()?;
assert_that!(
    drawn.mode(),
    eq(Mode::Played(Ending::Draw(DrawReason::Threefold)))
);

// Two bare kings cannot mate: the game ends by itself.
let dead = king_vs_king();
assert_that!(
    dead.mode(),
    eq(Mode::Played(Ending::Draw(DrawReason::InsufficientMaterial)))
);
```

## Decisions (the part worth writing down)

### 1. Draws are the first rules that live above the position

A memoryless `Position` cannot know it has occurred three times.
Repetition and the move-count rules are facts about the *history*, so
they belong to `Game` — the layer that already memoizes every position
the log produced. Threefold detection is counting equal entries in
`history`; the memoized fold pays for the third time. The one
exception: insufficient material (FIDE's dead position, by the
standard material table — K vs K, K+B vs K, K+N vs K, K+B vs K+B with
same-colored bishops) is a fact about the *board*, so it joins
checkmate and stalemate in position-level `mode()`.

### 2. Derived, never stored — the halfmove clock too

The fifty-move rule counts plies since the last capture or pawn move.
FEN stores that as a counter; we don't have to, because the log knows:
`Game::quiet_plies()` walks `history` backwards until it sees a pawn
move or a capture. Same thesis as `mode`, `spent`, and `ended` — no
state to keep consistent, nothing for `undo` to repair. (When FEN
import/export lands, the counter is computed on export; importing a
mid-count FEN is the one place this costs anything, and that plan can
carry an explicit override.)

### 3. "Same position" is a domain equivalence, not structural equality

FIDE 9.2: positions repeat when the same player has the same pieces on
the same squares with the same castling rights *and the same en
passant possibilities*. Our `passant` field records the skipped square
even when no enemy pawn can actually capture there — structurally
different, FIDE-identical. So repetition does not compare `Position`
values directly; it compares a normalizing `repetition_key()` that
counts the passant square only when an enemy pawn stands ready to take
it (pseudo-legal adjacency, the arbiter's practice). Derived `Eq`
stays simple and total; the domain's coarser equivalence gets its own
name. That distinction — structural vs domain equality — is the
chapter-worthy bit.

### 4. Armed vs automatic — the flag-claim pattern again

FIDE splits draws exactly the way the clock split:

- **Claimable** (a player must ask): threefold repetition, fifty-move.
  `Game::claim_draw()` — succeeds with `Ending::Draw(reason)` when a
  claim is earned, else `Rejected::NoDrawToClaim`. An unclaimed
  threefold keeps the game `Playing`, like an unnoticed flag.
- **Automatic** (the arbiter ends it): fivefold repetition,
  seventy-five-move, insufficient material. `Game::apply` derives them
  into the memoized mode the same way mate arrives — by itself.

The position arms the draw; the claim fires it. Nothing happens by
itself, except where FIDE says it must.

## Stages

1. `Ending::Draw(DrawReason)` with
   `DrawReason::{Threefold, FiftyMoves, Fivefold, SeventyFiveMoves, InsufficientMaterial}`;
   insufficient-material detection in position-level `mode()`.
2. Game-level derivations, public: `quiet_plies()` (run since capture
   or pawn move) and `repetitions()` (occurrences of the current
   `repetition_key` in history).
3. Automatic endings in `Game::apply`: fivefold and seventy-five-move
   fold into the memoized mode.
4. `Game::claim_draw()` for threefold and fifty-move;
   `Rejected::NoDrawToClaim` when unearned.
5. Scores and imports: drawn games emit and verify `1/2-1/2`.
6. Tag `v0.6.0`; check the checklist item.

## Explicitly deferred

- Draw by agreement and resignation: player events, not board facts —
  trivially `Ending` variants later, and PGN import already accepts
  the result markers the board cannot verify.
- True dead-position analysis (FIDE 5.2.2 beyond the material table):
  "no sequence of legal moves mates" is search, not a rule table.
- FEN import/export: decision 2 sets it up; its own small plan.
