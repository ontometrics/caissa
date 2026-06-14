# Plan: variations (v0.8.0) — the `study` module

The first namespaced subsystem (core stays flat at the root; `study` is
a bundle you opt into, like `notation`/`pgn`/`classics`). The goal,
stated as the test that ends the work:

```rust
use caissa::study::Study;

// The mainline, with an alternative branching at White's third move.
let study = Study::from(ruy_lopez())          // e4 e5 Nf3 Nc6 Bb5
    .branch(&[2], "Bc4")?;                      // 3. Bc4 instead of Bb5

assert_that!(study.mainline().score().contains("3. Bb5"), eq(true));
assert_that!(study.line(&[2, 0]).unwrap().position(), eq(/* after 3. Bc4 */));
```

## Decisions (the part worth writing down)

### 1. A layer above `Game`, never inside it

`Game` stays a linear fold. A `Study` is `Game` + branches, the way
`Timeline` and `Clocked` are `Game` + a concern. Every law already
proven on `Game` stays untouched; variations are purely additive.

### 2. The tree *is* the prefix sharing

The vision's "a study is a tree of logs sharing prefixes" becomes
literal: a move tree shares prefixes **structurally**, not as an
optimization. Two lines that agree through ply *n* are one node
sequence up to the branch, and divergence is a new child. Nothing is
copied; sharing is what a tree *is*. (Contrast the rejected "a
variation is a `Game` forked at a ply," which copies the prefix into a
fresh log.)

Each node stays faithful to the crate's discipline — it stores the
`Action` (the truth) and the resulting `Position` (the memoized fold),
exactly as `Game` does. Derived, never stored.

### 3. Mainline is the first child

One uniform rule, matching how `game[ply]` and `Terminus` already treat
"first" as canonical: at every node, child 0 is the mainline
continuation; later children are variations, recursively. No privileged
"mainline vs variation" type — just a convention on order, so a
variation of a variation needs no new concept.

### 4. Every line projects back to a `Game`

`study.line(path) -> Game` folds the actions along a path into a plain
`Game`, so **all** the derived queries — `mode`, `score`, `fen`,
`captures`, `quiet_plies` — come free on any line, mainline or
variation. `study.mainline()` is `line` down the all-zeros path. This
is the layering paying off: the new structure borrows the old one's
whole vocabulary instead of reimplementing it.

### 5. A study keeps transpositions; the dictionary merges them

The dual worth stating. The dictionary (`HashMap<Position, _>`) keys on
the **value**, so transpositions collapse — many roads, one entry. A
study keys on the **path**, so two move orders reaching the same board
stay two nodes. Neither is wrong: the dictionary answers "what is true
of this position?", the study answers "what was the line?". Same
maze/Markov boundary as before, seen from the other side.

## Proposed surface (red-pen welcome)

```rust
pub struct Study { /* start: Position, lines: Vec<Node> */ }
struct Node { /* action: Action, position: Position, branches: Vec<Node> */ }

impl Study {
    pub fn new() -> Study;                         // from the standard start
    pub fn from(game: Game) -> Study;              // a game is a study with no branches
    pub fn from_position(start: Position) -> Study;

    pub fn branch(&self, at: &[usize], action: impl IntoAction)
        -> Result<Study, Rejected>;                // add a continuation at a path
    pub fn line(&self, path: &[usize]) -> Option<Game>;  // the game along a path
    pub fn mainline(&self) -> Game;                // the all-zeros line
    pub fn variations(&self, at: &[usize]) -> usize;     // how many children there
}
```

Navigation is by **path** (`&[usize]` of child indices from the root) —
plain, `Copy`-friendly, and it mirrors jump notation. A functional
zipper (focus + context) is the alternative; it is more elegant for
heavy interactive editing but heavier than this feature needs now.
Pinned as a possible later cursor type, not v0.8.

Naming forks for you: `Study` (module `study`, so `study::Study` —
mild stutter, common enough) vs `study::Tree` vs `study::Line`; and
`branches` vs `children` vs `continuations` for a node's successors.

## Stages

1. The tree: `Study`, `Node`, `branch`, `line`, `mainline`,
   `variations`. Building and navigating programmatically. Tests:
   `Study::from(game)` round-trips its mainline; a branch at a ply is
   reachable and its line projects to the right `Game`; a sub-variation
   nests; an illegal branch action is rejected with the reducer's own
   verdict.
2. PGN variation **import**: turn today's "variations rejected loudly"
   into a recursive-descent parse of nested `(...)` into the tree.
3. PGN variation **export**: a study writes `(...)` — the inverse, the
   mirror of stage 2.
4. Tag `v0.8.0` (stages 2–3 may slip to `v0.9.0` if the parser wants
   its own release).

## Explicitly deferred

- Annotations as data: comments (`{...}`) and NAGs (`$n`) attached to
  nodes. The PGN parser already skips them; storing them is a separate
  concern (a node gains an annotation field — or, in keeping with the
  thesis, annotations ride a parallel structure).
- Promoting a variation to the mainline (reordering children) — an
  editing operation, easy once the tree exists.
- The zipper cursor, if interactive editing ever needs it.
- Transposition links across branches — deliberately *not* done;
  that's the dictionary's job (decision 5).
