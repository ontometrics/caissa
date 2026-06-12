# caissa: vision and vocabulary

caissa is a functional chess library: positions are values, moves are
pure transitions, a game is a fold. It is rules-complete as of v0.6.0.
This document is the decision record above the code — the load-bearing
vocabulary, and the big questions that decide where the project goes
next. The plan docs beside it (`san-pgn-plan.md`, `draw-rules-plan.md`)
record per-feature decisions; this one records the project's.

## Vocabulary

**Value semantics.** `Position` is a plain `Copy` value (~130 bytes).
Every transition yields a new position and the old one stays valid.
History, undo, search trees, and variations are all the same operation:
*keep the old value*.

**The fold.** A game is `actions.try_fold(start, reduce)`. Replay,
import, and undo are corollaries, not features.

**The log is the source of truth.** `Game` is a starting position plus
the log of accepted actions — morally a PGN, literally a write-ahead
log. Everything else is computed from it.

**The memoized fold.** `Game.history` caches every intermediate
position the log produced. It is a cache, never an authority: replaying
the log reproduces it. Jump notation (`game[n]`, `game[Terminus - 1]`)
indexes it; `undo` truncates it.

**Derived, never stored.** The project's strongest discipline. `Mode`
(playing/played), clocks (`spent`, `remaining`), the halfmove counter
(`quiet_plies`), repetition counts, the capture tray — none of these
are fields that can drift; all are folds over the log or scans of the
board, computed when asked. When tempted to add state, derive instead.

**Prefix sharing.** Two games that agree through ply *n* have identical
logs and histories up to entry *n* — the same values. Because positions
are values, "sharing" costs nothing semantically: a variation is a new
game built from a prefix of an old one (`undo` is already the trivial
case — sharing all but the last entry). Today the prefixes are copied
(cheap at these sizes); a future variation tree could share them
structurally (persistent vectors), changing performance, not meaning.
This is the concept the variations feature will turn into API: a study
is a *tree of logs sharing prefixes*, exactly how opening theory is
shaped.

**The interpreter.** `Action` is surface syntax (a from–to intent).
`expand` validates it against the position and compiles it from its
*prototype* into primitive `Edit`s (`Lift`, `Place`); `apply` is the
total evaluator that folds edits over the board. Castling's prototype
is four edits; nothing is special after expansion. Captured pieces
simply cease to exist when lifted — the capture tray is derived.

**Ply.** A half-move, one action by one player; the fold's natural
step. Publications' "move N" is two plies; `score()` does `ply/2 + 1`.

**Domain equivalence vs structural equality.** `Eq` on `Position` is
structural and total. FIDE's "same position" (repetition) is a coarser,
domain-specific equivalence — `repetition_key()` counts the en-passant
square only when a pawn can actually use it. When a domain disagrees
with `derive(PartialEq)`, name the domain's notion; don't bend the
structural one.

**Armed vs automatic.** Some endings need a player to ask: an unclaimed
flag is no flag, an unclaimed threefold keeps playing (`claim_flag`,
`claim_draw`). Some end the game by themselves: mate, stalemate,
fivefold, seventy-five-move, dead position. The position arms; the
rules say who fires. Nothing happens by itself unless FIDE says it must.

**Time as data.** The core never reads a clock. `Timeline` annotates
the log with stamps (generic — integers in tests, `Instant` in
production); `started`/`ended` form a Snodgrass valid-time interval
(`None` = still playing); `Clocked` adds budgets and flags. A replay is
a value; the only effect is the `sleep` between frames.

**The layer stack.** `reduce` (legality) → `Game` (memory: history,
mode, draws) → `Timeline` (chronology) → `Clocked` (obligation). Each
layer adds one concern, built from the one below, and each ending is
honest about which layer produced it.

## The horizons

Three questions, in increasing order of ambition. The crate's answers
should preserve its identity: the core stays a pure, dependency-free
rules engine; ambition arrives as layers and siblings, not as fields.

### 1. Does it play?

Yes, eventually — and the architecture already contains the shape.
Search is *the fold run speculatively*: `legal_actions()` enumerates
the branches, value-semantic positions make tree nodes free (keep the
old value), and `mode()` is the leaf evaluator that never lies. A
player is just a function `Position -> Action` choosing among legal
actions; the watcher loop from the clock tests is already the game
driver. Stages, each useful alone:

1. a random legal player (one line; makes self-play loops real),
2. material + mobility evaluation with shallow minimax,
3. MCTS, where persistence genuinely shines.

The cost that decision buys: the reducer pays an interpreter's price
per move (fine for humans, ruinous at engine depth). The long-reserved
*engine fast path* — ungated movegen, possibly bitboards — becomes
worth building exactly when search arrives, as a basement under the
same API, never as the front door.

### 2. Can it learn?

The crate's role in learning is *environment, referee, and data* — not
neural network. It can already do the three things a learning loop
needs: generate legal experience (self-play via the fold), judge
outcomes (`mode()` never lies), and serialize history (the log).
Candidate shapes, smallest first:

- **Evaluation tuning** (Texel-style): fit a linear/simple evaluator's
  weights against game outcomes from PGN corpora — pure data-fitting,
  no new infrastructure.
- **Self-play improvement** (AlphaZero-shaped): policy/value function
  guides MCTS; games it plays become training data. caissa is the
  environment; the function approximator lives in a sibling crate
  (`caissa-learn`?) so the core keeps zero dependencies.
- **Sequence modeling**: treat games as token streams and learn the
  way language models do — which makes the third question the
  interesting one.

### 3. Which encodings make learning powerful?

The thesis extends one more step: **an encoding is just another fold
over the log.** The same game projects into many representations, all
derivable today, none stored:

- **Action stream** — from–to tokens (~the AlphaZero move space).
  Compact; castling and promotion are opaque conventions.
- **Edit stream** — the interpreter's instruction trace: `Lift(sq)`,
  `Place(sq, piece)` — a vocabulary of ~832 symbols in which captures,
  castles, promotions, and en passant are *visible structure* instead
  of special cases a model must infer. A game becomes a sentence in the
  board's own assembly language. This one is unusual — possibly the
  most caissa-native contribution to the question.
- **SAN stream** — the human prior baked into notation ("Nbd7" encodes
  role, destination, and ambiguity); what existing chess-LLM work reads.
- **Board planes per ply** — 12×8×8 tensors straight from `history`,
  for spatial models; the memoized fold makes this a map, not a replay.
- **Think-time-annotated streams** — `Timeline` frames carry how long
  each move took: a difficulty/confidence signal no standard chess
  encoding preserves. Genuinely novel training signal, and we already
  have it.

The right move is not to pick one but to make caissa the *encoding
laboratory*: one game, many projections, measured against each other
on the same learning task. The log being the single source of truth is
what makes the comparison fair.

#### Phrases: compression as pattern discovery

The conjecture (Rob): LLMs work as well as they do because context
ballooned — a token is never just "I saw this word"; it is seen with
its position, inside its phrases. The chess parallel should exist:
patterns compressed into tiny sequences that the learning gloms onto.

The mechanism with a name: **byte-pair encoding**. BPE knows nothing
about words; it statistically discovers frequent subsequences in a
corpus and compresses each into one token — which is exactly "find the
phrases and make them units." And chess has already chunked itself by
hand: openings have *names*, ECO codes are a phrase dictionary, a
tabiya is a known-position chunk, "fianchetto" and "recapture" are
idioms. Centuries of players don't name the arbitrary — the names are
evidence the compressible patterns exist. We hand-compressed one
ourselves: castling is four `Edit`s wearing one name.

The first encoding-lab experiment, fully concrete: run BPE over a
large PGN corpus projected as Edit streams (and, for comparison,
action streams and SAN streams). The learned merge table *is a
discovered phrase book of chess*. Two measurable questions follow:

1. **Alignment** — does the discovered phrase book rediscover the
   human one? Do merges converge on recapture pairs, fianchettos,
   whole ECO opening systems? Where machine chunks and human names
   disagree is exactly where something interesting lives.
2. **Leverage** — do sequence models trained on phrase tokens learn
   faster or play better than ones trained on raw move tokens, holding
   the corpus fixed? That is the ballooned-context conjecture, run as
   an experiment.

The derived queries extend the idea: check, capture, mode, and
think-time can ride along as cheap semantic markup — the
part-of-speech tags of chess — so the context isn't just bigger, it's
annotated. The Edit stream is the natural substrate for all of this
because its vocabulary is small and its compound moves are visible
structure: the phrases have something honest to be made of.

#### From phrases to plans: invariances and the key family

The worked example (Rob): Kasparov played the English Opening a
bazillion times. Are openings about mounting an attack or preparing a
defense? Both — and that duality is the point. An opening's purpose is
to steer the game into *structures whose plans you know better than
your opponent does*; attacking and defensive plans are cached against
the same key. In this project's vocabulary: **theory is prefix sharing
at cultural scale** — a shared prefix tree the whole chess world
maintains — and a repertoire is one player's deeply-practiced subtree.
Preparation is memoization of plans, keyed by structure.

"Can we see similar attacks across his games? Of course" — and that
*of course* is where BPE stops. BPE discovers contiguous idioms;
Kasparov's recurring attacks transpose: same plan, different move
order, shifted a file, a tempo later. The sequences differ while the
structure trajectory recurs. The caissa-native answer is the tool we
already built for draws, generalized: **a family of coarsening keys**.

    Eq  ⊃  repetition_key  ⊃  pawn-structure key  ⊃  material key

Each coarser equivalence reveals a different recurrence: structural
equality is identity; `repetition_key` finds repetitions (draws) and
transpositions (opening books); a pawn-skeleton key — pawns only, the
slowest-changing layer of the position — finds *plans*, because two
games share a middlegame plan exactly when they share a structure,
whatever the piece dance above it looked like. Strategy lives in the
quotient spaces.

Player-conditioned corpora make it style: tokenize a player's games
and the phrase distribution is a signature — preparation made visible.
Measurable experiments: cluster Kasparov's English games by
pawn-skeleton trajectory and see whether the famous attacks cluster
with them; train an attribution model and ask whether the phrase book
alone identifies the player. The encoding lab's claim sharpens to:
phrases (contiguous, BPE-discoverable) capture *tactics and book*;
recurrences modulo coarser keys capture *plans*; a powerful encoding
carries both.

## Sequencing

Near-term (the checklist): v0.7.0 interchange (FEN + tagged PGN
export), then variations — where prefix sharing becomes API — then the
book staging, while the decisions are fresh. The horizons come after,
as layers and siblings: player, then learner, then the encoding lab.
The book gets chapters out of every stage; the tags mark where each
one stands.
