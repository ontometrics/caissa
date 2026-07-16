# Plan: variations (v0.8.0) — the `study` module

The first namespaced subsystem (core stays flat at the root; `study` is
a bundle you opt into, like `notation`/`pgn`/`classics`). The goal,
stated as the test that ends the work:

```rust
use caissa::study::Study;

let mainline  = ruy_lopez();                     // e4 e5 Nf3 Nc6 Bb5
let variation = mainline.undo().apply("Bc4")?;   // …Nc6 3. Bc4 instead

// Graft a whole line; the tree discovers where it diverges.
let study = Study::from(mainline).with(variation);

assert_that!(study.mainline().score().contains("3. Bb5"), eq(true));
assert_that!(study.lines().count(), eq(2));
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

And the sharing is the *construction* mechanism, not just the storage:
you build a study by grafting whole lines (`with(line)`), and the tree
walks each new line from the root, coincides along the shared prefix,
and branches where it diverges. The branch point is discovered, never
addressed. The crate composes everything by folding values
(`actions.try_fold(start, reduce)`); a study composes the same way, by
absorbing games. No path of child-indices ever surfaces — that would be
an address into the structure, the board-as-raw-indices smell the crate
avoids everywhere else.

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

Build by grafting lines; read by extracting lines. No path of indices
appears anywhere — the tree is internal.

```rust
pub struct Study { /* start: Position, branches: Vec<Node> */ }
struct Node { /* action: Action, position: Position, branches: Vec<Node> */ }

impl Study {
    pub fn new() -> Study;                  // the standard start, no moves
    pub fn from(game: Game) -> Study;       // a game is a study of one line
    pub fn from_position(start: Position) -> Study;

    pub fn with(self, line: Game) -> Study; // graft a line; the shared prefix
                                            // merges, first line added is mainline
    pub fn mainline(&self) -> Game;                       // the first-child line
    pub fn lines(&self) -> impl Iterator<Item = Game>;    // every line, mainline first
}
```

You speak in games at every turn: a variation is built from the
mainline with the `Game` vocabulary you already have (`undo`, `apply`),
then absorbed whole. Reading gives games back, each carrying all the
derived queries (decision 4). The only contract is that grafted lines
share the study's start — they are variations *of one game* — which is
automatic when you build them from the mainline or import them from one
PGN.

Deferred to a later cursor: single-move editing at a focus point
(add/delete/promote one move). That is what tempted the `&[usize]`
path; it belongs to a functional zipper (focus + context), not the
high-level surface, and only matters once interactive editing does.

Naming forks for you: `Study` (module `study`, so `study::Study` —
mild stutter, common enough) vs `study::Tree` vs `study::Line`;
`with(line)` vs `and(line)` vs `add(line)` for grafting; and `branches`
vs `children` vs `continuations` for a node's successors.

## Stages

1. The tree: `Study`, `Node`, `with`, `mainline`, `lines`. Building by
   grafting, reading by extracting. Tests: `Study::from(game)`'s
   mainline is that game; grafting a variation yields two lines sharing
   their prefix; a sub-variation (a variation off a variation) nests; a
   line that diverges later branches deeper, not at the root; the first
   line added stays the mainline.
2. PGN variation **import**: turn today's "variations rejected loudly"
   into a recursive-descent parse of nested `(...)` into the tree.
   **Shipped v0.13.0** — `pgn::import_study`, per the design below.

   Design (settled with Rob): this is the moment the anonymous lexer
   gets its name. The pipeline inside `pgn::parse` (comment-stripping
   state machine → whitespace split → move-number stripping) already
   *is* a lexer — raw text to clean SAN tokens — but today it *rejects*
   `(`, the one character that would make the language context-free.
   Stage 2 flips that: the lexer emits parens as structural tokens, its
   output growing from strings into a token enum —
   `Token::{San(String), Open, Close, Result(String)}` — and the parser
   side is the single recursive function already scoped (consume
   `&[Token]`, recurse on `Open`, return on `Close`, graft via `with`).
   The lexer/parser boundary is the regular/context-free boundary: a
   finite state machine for the regular sublanguage, one recursion for
   the one context-free production. No combinator library (see the
   voided IOU in san-pgn-plan.md); the pipeline is
   text → tokens (lex) → tree (parse) → `Study` (fold) — the same
   interpreter shape as `Action → Edits → Position`, one altitude over
   text.
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
