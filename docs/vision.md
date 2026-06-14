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

**Laws have one address.** The complaint this project answers (Rob):
software implements common knowledge *residually* — a rule's
enforcement smeared across call sites, with no single place where the
law is stated. Castling is the worked counterexample. "Rights only
ever shrink; the king's first step forfeits both wings forever" is
*stated* once (the doc comment on `Rights`), *enforced* once (the
evaluator's per-edit bookkeeping — the one door every move walks
through), and *witnessed* once (tests named for the law). Three
mechanisms make one-address laws possible: a **choke point** (all
state change compiles to `Edit`s through one `apply`; laws enforce at
the door — before the interpreter refactor this law had two addresses,
and the refactor's real product was collapsing them); **law as
absence** (`Rights` has `clear` and no `set` — "nothing restores them"
is a path that doesn't exist, the strongest enforcement at zero
runtime cost); and **derivation** (a stored consequence can drift from
its law; a derived value *is* the law). The smell, stated crisply:
when a rule has to live in a wiki to survive, its address in the code
is "everywhere."

The three mechanisms aren't independent tricks — they are what value
semantics *buys*. A choke point is only trustworthy when there is no
other way to mutate: values can't be aliased and scribbled on behind
the law's back. Law-as-absence only works when the type's API is the
complete set of state transitions. Derivation is only safe when the
source of truth can't drift. Mutable-object designs leak on all three
at once — every setter is a new address — which is why their laws end
up residual: the true statement degrades into tribal knowledge and
wiki pages. This is the actual argument for the functional
architecture: not elegance, *jurisdiction*.

**Compose, don't address.** An interface should take domain values, not
coordinates into its own data structure. The board makes you say
`Square`, never `board[19]`; a study makes you graft a `Game`, never
name a node by a path of child-indices. A raw index — `&[usize]`, an
array offset, a node id — is plumbing: it forces the caller to know the
internal shape and leaks the structure through the surface. The cure is
always composition: build by folding or grafting values and let the
structure discover where they fit. When `study.with(line)` replaced the
proposed `branch(at: &[usize], action)`, prefix sharing stopped being a
storage property and became the *construction* mechanism — the
divergence point is discovered, not specified. And reaching for an
address is usually a sign you pulled a low-level operation up too early:
single-move editing wanted that path, and it belongs behind a proper
cursor (a zipper — focus + context), not on the high-level surface.
Same family as the residual-primitive smells (`&mut` in `Rights`, the
`Option` beside `Change`): a surface that hadn't finished speaking the
domain's language.

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

**Self-play is headless by nature (Rob).** A bot is a pure function of
state — it reacts to the position, needing no view — so the simulator
is just a loop: ask the side to move for an action, fold it, repeat
until `mode()` is `Played`, and out comes a `Game`. No rendering, no
I/O. This is not only tidy, it is *required*: the corpus flywheel that
feeds learning, the dictionary, and the encoding lab wants millions of
games, and a rendering loop would be a thousand times too slow
(AlphaZero never watches its own games either). Headlessness is the
price of the data ambitions, paid for free because the model never knew
a view existed. The Markov property licenses it: the position is
sufficient to choose a move, so a bot needs the *state*, never the
*screen*. The view is a separate, optional consumer — feed the produced
`Game` to `Timeline` and replay it with think-times only if a human is
watching; rendering is the one effect at the edge, the `sleep` between
frames, which is exactly why `Display` and the rules have always lived
in different files. (A *complete* bot takes the `Game`, not just the
`Position`, since the non-Markov rules — repetition, the fifty-move
clock, draw claims — need the sliver of history `Game` carries; a
simple bot reads only `.position()`. Either way, no view.)

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
  `Place(sq, piece)`, `Skip(sq)` — a small vocabulary in which captures,
  castles, promotions, and en passant are *visible structure* instead
  of special cases a model must infer. A game becomes a sentence in the
  board's own assembly language. The stream is *complete*: a move is
  exactly its edits, the en-passant window included (the `Skip` verb
  exists precisely so the stream hides nothing — an earlier design
  carried that window in a field beside the edits, and a model reading
  only the stream would have missed which double pushes armed en
  passant). This one is unusual — possibly the most caissa-native
  contribution to the question.
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

### 4. Where the game tips: value, shifts, and the inevitable

The precedent (Rob): Ed Thorp cracked card counting with a simulator —
remove each card from the deck, rerun the million-hand simulation, and
the outcome deltas yield a point value per card, for player and house.
Perturb, simulate, measure: value emerges from counterfactuals at
scale. Chess is simpler in one way — there is no hidden deck. Every
position already has a number: **W(position)**, the likelihood either
side wins. The player horizon builds its estimator for free (W is what
rollouts measure — by simulation first, by a learned function later),
and the fold gives everything downstream:

**The shift series.** Map W over `history` and difference it: ΔW per
ply — derived, never stored, like everything else. Most plies barely
move it; a handful move it violently. *Knowledge should come from
understanding where the shifts occurred and why* — a game's
information is not uniform over its plies, and the shift series says
exactly where it lives.

**The inevitable is measurable.** A lot of chess is unfolding the
inevitable after one side gained an advantage that is nearly
impossible to unwind. Operationally: inevitability is the fraction of
playouts in which the disadvantaged side still saves the game. When W
pins near 1 and playout variance collapses, the rest is execution —
low-information plies, almost fully compressible. This re-grounds the
encodings section from the other side: the incompressible kernel of a
game is its shift points; weight tokens by information, train on the
moments, skim the unfolding.

**Why, by ablation.** Thorp's move, translated. The cause of a shift
is isolable counterfactually: re-simulate from the road not taken (the
played move's regret against the best); ablate at the structure level
(which pawn, removed, restores W?). Per-card point values become
per-piece, per-structure values — position-specific instead of
folklore (the 1/3/3/5/9 piece values were early, crude estimates of
exactly this quantity). The *why* a coach speaks is ΔW attributed in
the key-family vocabulary: the shift at ply 23 happened because the
skeleton changed into one you don't know the plans for.

The closing symmetry: a human player's running judgment — material
count plus structural heuristics — is the cheap on-line approximation
of W, exactly what the card counter's running count is to the
simulator's true values. Chess skill is, in part, carrying a good
estimator of W in your head; the coach's job is improving the
student's estimator, and to do that it needs the real one.

#### The Mencken trap and the Bellman answer

The mind-bender (Rob): if every position has split odds, then a move
is simply the one that reaches the position strongest for your side —
but that's quick, simple, and wrong (Mencken), because the beauty of
chess is accepting a decline now in favor of a larger goal a few steps
down the road.

The resolution is sharper than either horn: **greedy on the true W is
optimal** — Bellman's principle. The true value function is the fixed
point of its own law: W(p) equals the best, over legal moves, of the
opponent-flipped W of the successor. At the fixed point there are no
sacrifices. When Anderssen gave away both rooks and the queen, the
*truth never declined* — the Immortal Game's W stays pinned while the
material count collapses four times. What declines in a sacrifice is
the estimate, never the value. **A sacrifice is an estimator
illusion.**

So the Mencken trap is real, but it lives in the gap **W − Ŵ**: every
playable estimator — material count, structural heuristics, a trained
network, the human running count — is wrong somewhere, and "accept a
decline for a larger goal" is the phenomenology of trusting the truth
against your own estimator. The "few steps down the road" is exactly
the horizon at which the estimator becomes reliable again, and search
is the repair mechanism: minimax to depth d is greedy on a better
estimator built by unrolling. Depth substitutes for accuracy.
Courage, in chess, is W − Ŵ arbitrage.

Two consequences worth building:

- **Brilliance is measurable.** A brilliant move preserves W while
  cratering Ŵ for the audience's estimator class. Run *two*
  trajectories over a game — the material estimate and the rollout
  truth — and their divergence locates the sacrifices. The Immortal
  Game is the canonical test: the material line dives four times; the
  truth line never flinches. (The material trajectory is computable
  today — it is a fold over the capture tray; the truth line awaits
  the player horizon's rollouts.)
- **The Bellman equation is a law with one address.** W's
  self-consistency is checkable everywhere: tablebases satisfy it
  exactly, estimators violate it, and the violation — the Bellman
  residual — is both where search helps most and what
  temporal-difference learning minimizes. Learning is the law
  enforcing itself.

The coach inherits the deepest version: teaching is shrinking the
student's W − Ŵ gap, and "you should have played the sacrifice" is
precisely the claim that the student trusted their running count where
it was wrong.

**The Moneyball refinement.** The rebuttal that turned out to be the
mechanism (Rob): maybe the model was just focused on material and
never saw that, in acquiring it, one side was digging itself a fatal
hole — a better-trained model would have seen the outcome coming. Yes:
the W − Ŵ gap is not mystical, it is *feature-shaped*. Kieseritzky was
the scout maximizing the legible statistic while the illegible ones —
king in the center, sleeping pieces, opening lines — cratered. The
hole was visible the whole time, in features he wasn't weighing. The
history of chess ran the full Moneyball arc: the Romantic era was a
market inefficiency (material priced too high, gambits systematically
underpriced, Anderssen arbitraging the gap), and then Steinitz played
Billy Beane — positional principles were the better feature set, the
market repriced, the romantic gambits stopped working. Brilliance is a
market inefficiency, and like all of them it gets arbitraged away.

The limit that keeps it interesting: under bounded compute the gap
shrinks and *migrates* but never vanishes — some positions are
irreducibly deep, which is why even the strongest learned evaluators
still search. So brilliance is relative to an estimator class, and the
detector upgrades accordingly: a move's brilliance is **the rank of
estimator required before it stops looking like a sacrifice** — run
the trajectory under a ladder of estimators (material only; plus
development; plus king safety; plus shallow search; …) and record
where the illusion dissolves. Era-relative brilliance falls out for
free: what stunned 1851 is an instant recapture today.

This closes a loop with the encoding laboratory: features *are*
encodings — the phrase book and the key family are candidate features
— so discovering better encodings and shrinking the estimator's blind
spots are the same research program. And the coach's job restates one
final time: not just shrinking the student's gap, but *installing
features* — teaching the on-base percentages of chess.

#### The dictionary: snapshot is the default

Where does W live? (Rob): in an OO app you would build a board, place
pieces, mutate — *snapshot* would be a feature to design. Here it is
the default: every `Position` is already a snapshot, `history` is a
list of them, and `Position` is `Hash` — so a dictionary of boards
with their W/B tallies is one fold away (`dictionary_test.rs`
demonstrates it on the classics). Three properties make it more than
a cache:

- **The key family is the resolution dial.** Keyed by `Eq` it is a
  cache; keyed by `repetition_key` transpositions merge — which is
  what makes it an opening *book*; keyed by pawn skeleton it becomes a
  *plan book*, W per structure.
- **Tallies are a monoid.** W/D/L counts add component-wise, so
  dictionaries merge: building one is an incremental, parallelizable
  fold over any corpus, and a student's personal book merges with — or
  measurably diverges from — the world's. Where the student's book
  runs out is where their hesitation should spike; the coach can
  correlate the two.
- **The three regimes are one idea.** Endgame tablebases are the exact
  dictionary, complete where the domain is small (≤7 men is solved);
  opening books are empirical tallies on the manifold games actually
  visit (legal chess is ~10⁴⁴ positions, but played chess
  Zipf-concentrates onto a sliver); and a learned value function is
  the dictionary *compressed* — what you store when you can't store
  it. Store the visited, learn the rest, and let FEN (v0.7) be the
  permanent key.

**Memorize, compute, generalize (Rob).** The laziest engine is "play
the most common move for this position" — a frequency table keyed by
the board. That is not the LLM answer; it is the *pre*-LLM answer, the
n-gram model, and it fails for the reason it always fails: too many
positions. Legal chess is ~10⁴⁴ states, so the table is unrealizable;
it works only on the Zipf sliver games actually visit (which is exactly
why opening books work — and stop working the moment you are "out of
book," staring at a position no table ever saw). The tail is unbounded;
the table is finite. So a position has three kinds of answer, and they
are the three regimes wearing player-hats:

- **memorize** — a book; answers only the visited sliver (the dictionary
  keyed by `Eq`, every position an island);
- **compute** — search; answers any position by working it out from the
  rules, no memory needed (`Minimax`/negamax never saw the hanging
  queen, it derived the capture);
- **generalize** — a learned evaluator; answers a novel position by
  interpolating from feature-similar ones it did see.

"Too many positions" is the wall that forces the handoff from the first
to the other two. And the bridge is, once again, the key family:
keyed by `Eq` the dictionary is the unrealizable island-table; keyed by
a *coarser* key (the pawn skeleton) it is both realizable and
generalizing — positions sharing a skeleton share an entry, so a novel
one inherits what its lookalikes learned. *Coarsening the key is lossy
compression is generalization* — the dial we built for repetition is
also the dial between memorizing and learning. A real engine stitches
all three by phase: book in the opening, search in the middlegame,
tablebase in the endgame.

**The council: blend the strategies, shift the weights (Rob).** Better
than stitching by phase is a weighted *blend* whose weights track which
expert is competent right now — the modern architecture: AlphaZero
blends a learned policy with search (the policy weights which branches
search explores; search corrects the policy), Stockfish blends search
with a learned eval. Three things make it cohere here:

- **It is composition.** Each strategy is a `Player`; an ensemble that
  weights several is *itself* a `Player`, closed under the same
  interface — a council of councils is a council. Free, once the
  members exist.
- **Blending needs opinions, not verdicts.** You cannot average two
  argmaxes. To weight strategies each must report *scored moves* — a
  value per move — not one chosen action, so the ensemble refines the
  interface (`assess(&Game) -> Vec<(Action, Score)>`, with `choose` the
  argmax of the weighted sum). Same shape as errors-as-data and
  `AmbiguousSan` returning candidates: expose the options, let the
  combiner decide.
- **W is the common currency.** Experts speak different languages — a
  book gives move frequencies, search centipawns, a learned eval
  win-probability — and cannot be averaged until they share units. W is
  the universal one: centipawns → W by a sigmoid, book frequencies → W
  by the outcomes the dictionary already tallies, a learned eval → W
  directly. The dictionary's W tally *is* the book-expert's output in
  the common currency. Everything reduces to estimates of the one
  number.

The weights should track *competence on this position*, not just phase
— a book move when the line is well-trodden, search when the position
is sharp, a learned eval when it is feature-similar to its training.
That confidence-gating is a feedback controller: the system carries a
model of its own components' reliability and shifts toward whichever is
right (Wiener again, the coach turned inward), and the gating is itself
learnable, keyed by the same features that drive generalization.

Sequencing: a north star, not the next build — extract, don't
anticipate. An ensemble of one real expert (today, search) frames
nothing. The council earns its keep once the members are genuinely
complementary, so the order is dictionary (the book) → a learned eval →
*then* the council that blends them. Build the experts first; the
weighting is the last, easy layer.

#### Offense, defense, and the few forms of attack (Rob)

A proposed council member: an *attack-focused* strategy. Black is
always a ply behind — responding to White's initiative — and chess's
depth is that both sides play offense and defense at once. But project
onto *just the attack* and the forms are few: the Ruy Lopez's attacking
plan recurs across thousands of games though no two boards match.

The forms are few for a reason: **attack is low-dimensional because the
target is fixed and the geometry is constrained.** There are only so
many ways at a king on g8 — the h-file, the a2–g8 diagonal, the
f7/h7/g7 sacrifices, the pawn storm, the back rank — so a few dozen
named schemes cover most real attacks. Defense is high-dimensional;
there are a thousand ways to be solid. That asymmetry is why attacking
chess is taught as named motifs and defense as principles, and it says
which projection is worth learning: the attack.

Operationally it is another coarse key in the family — project a
position onto its attacking structure (attackers, target, levers) and
discard the rest. Under that key the Ruy's thousands of boards become
*one form*, and the dictionary is small and dense: memorization works
on attacks even though it drowns on positions ("too many positions"
does not apply to "too few attacks"). The lazy lookup table, useless
keyed by board, is viable keyed by attack-form.

The tempo asymmetry is the **initiative**, a quantity that *flows*:
White holds it by default, Black neutralizes to seize it, the
counterattack is defense becoming offense. So offense and defense are
the two ends of one variable, and that gives the council its gate —
*attack-focused* and *defense-focused* as dual experts, weighted by who
holds the initiative now, the weight flipping at the counterattack. It
decomposes the evaluator itself: **Ŵ = attack_potential −
vulnerability**, one expert per term; a sacrifice is the attack expert
overruling the material expert because the initiative outweighs the
piece.

And it is a testable conjecture, not a flourish: cluster the *attacking
phases* of a corpus and count the clusters. If the forms are few they
will be few, and align with the named motifs; BPE over attack-projected
streams should discover the motif vocabulary on its own. The hard part
— defining the attack projection precisely — is real research, a north
star with the others. But it sharpens the nearer work with a directive:
do not only encode positions, encode attack-forms, because that is
where the dimensionality collapses and learning gets cheap.

#### Mobilization, variance, and the fallible opponent (Rob)

A board can be quantized by how much *force each side has mobilized* —
developed, committed, pointed at the fight. And mobilization is
double-edged: pull the queen out early and you may land a quick win or
suffer unrecoverable tragedy if the attack is refuted. Rommel: have a
plan, but the battlefield is mistakes and surprises, and the winner is
the one who capitalizes better.

This adds the axis the rest of the document was missing. Everything so
far has measured **W, the expected value**. Mobilization and Rommel are
about the **second moment, variance**. Two positions can share a W and
differ utterly in character — a quiet position (low variance, hard to
lose or to win big) versus a sharp over-mobilized one (the same W, but
+mate or −resign depending on whether the attack lands). W cannot tell
them apart; mobilization is a *variance driver*.

And Rommel indicts plain minimax, which assumes the opponent plays best
— the game-theoretic optimum. A real opponent *errs*, and the winner
capitalizes: **optimal versus exploitative play**, the oldest dichotomy
in game AI. The two axes are the two champions — Petrosian minimized
his *own* error probability (prophylaxis, low variance), Tal maximized
his *opponent's* (sacrifice, chaos, high variance) — same
W-maximization, opposite variance strategies, both winning against
different opponents.

The connection that redeems the lazy lookup table: the "most common
move for this position" frequency table, useless as a *player* (too
many positions), is exactly the *opponent model* — "what will this
opponent likely do here." Exploitative play is W (what is best) blended
with opponent-prediction (what they will actually do). The table's true
job was never to move; it was to model the enemy. Rommel needs it.

The clean result: **optimal risk appetite is a function of W.** Behind
(W < ½) you should *seek* variance — the sharp, mobilized position is
your path to the tail; ahead (W > ½) you should *kill* it — simplify,
trade, demobilize. That one rule yields all the folk wisdom: complicate
when losing, simplify when winning, swindle from lost positions.
Kelly-flavored — bet bigger when you need the variance — with the one
caveat that *is* the "unrecoverable tragedy": variance is not ruin. A
hung queen is an **absorbing barrier**; you cannot un-lose. So
mobilization that widens the distribution is one thing, over-extension
that courts ruin another, and you accept ruin-risk only when already
lost. That is why "queen out early" is punished — it reaches for the
absorbing barrier before earning the right to gamble.

Caissa hooks: mobilization is a cheap feature (committed force per
side), the *fuel* to attack-form's *shape*, entering the eval as a
variance term conditioned on coordination (mobilized-but-uncoordinated
is the tragedy). Variance is computable from the very rollouts that
give W — take the spread, not just the mean (sharpness *is* rollout
variance; we were halfway there with "inevitability = the fraction of
playouts the loser saves"). And the council gains a risk dial gated by
W: a mobilizing (Tal) expert up-weighted when behind, a simplifying
(Petrosian) expert when ahead. The coach completes here too — it does
not only shrink the student's W − Ŵ gap, it teaches risk management:
when to complicate, when to simplify. Defining "mobilization" precisely
and modeling the opponent are research, north stars with the rest; but
variance is the axis that makes the value model honest.

#### The annotator: a second interpreter that reads games (Rob)

The question: an interpreter that rolls through classic games to
*understand its own cases* — did Spassky fumble to Fischer, and on what
move? Yes, and it is a second interpreter one altitude above the first.
The move-interpreter (`expand`/`apply`) turns a move into board edits —
what happened on the squares. The annotator turns a *game* into a
narrative of decisions — what happened in the contest. It is the shift
series operationalized: fold `history` through a W-estimator, difference
it (ΔW per ply), and the plies where ΔW lurches against a player are
their fumbles. "On what move" is the argmax of that player's loss. A
fold, like everything here.

"Understand its own cases" is the deeper half — case-based reasoning
(the `cbr` crate's premise). Detecting the shift is not understanding
it; *classifying* it is. Each fumble abstracts into a **case** keyed by
position-type (the coarse key again): a back-rank oversight, a walked-
into fork, a missed in-between move. Accumulate cases and you get a
case base — and the attack-forms argument applies to mistakes too:
**there are few kinds of blunder.** So the base is small and learnable,
and the annotator understands a new fumble by recognizing it as an
instance of a known kind — the leap from "ΔW spiked at move 29" to
"Spassky did the thing he does."

The honest limit: the verdict is **estimator-relative.** To judge
Spassky you must see what Spassky saw; a material-only annotator flags
every sacrifice as a fumble — the Moneyball gap in reverse, brilliance
misread as blunder. A "fumble" is relative to the estimator class,
exactly as brilliance was, so a weak annotator pointed at the masters
reveals its *own* blind spots (why 1990s engines "disagreed" with
grandmasters). Hence report confidence: clear fumbles — big ΔW a modest
estimator sees, a hung queen, a one-move tactic — are reliable and
teachable; subtle ones need a stronger judge and should say so rather
than pronounce.

It completes the coach by being one tool pointed two ways: annotate the
masters to *teach* the case-types, annotate the student's own games to
*diagnose* which cases they fall into. A fumble is the W − Ŵ gap
localized to a ply; the case base is the catalog of how the gap
manifests; the lesson is "here are your three recurring cases."

And it is unusually near-term: the clear-blunder annotator is buildable
now — `material` and `Minimax` are W-estimators, `history` and the
`classics` are here, so `annotate(game) -> Vec<(ply, delta_w)>` is a
fold plus a diff, and flagging the spikes finds a game's turning point.
Point it at a classic and watch. The CBR case layer is the north star
(it needs the position-type keys); the annotator's skeleton is a real
next brick.

#### Bad games teach the boundary (Rob)

Common knowledge says random, non-thinking games played to a result are
useless data — real players know not to play garbage, so garbage cannot
teach. That dismissal is the common-knowledge pit itself (Peirce's *a
priori* method: belief fixed by what is agreeable to reason rather than
tested). Tested, it is backwards: bad examples are essential, and
learning what *not* to do is the first stage. (The author of this
document made exactly this error building the v0.9 random player —
called the corpus "weak" — which is why the principle is recorded here.)

The rigorous core: **learning is locating a boundary, and a boundary
needs points on both sides.** A value function's job is to find the
cliff edge of W where winning becomes losing. Expert games *hug the
good side* — two strong players never walk off the cliff, so they never
show where it is; train only on them and the boundary is
underdetermined. Bad games map the cliff by falling off it everywhere.
You cannot learn "do not hang the queen" from games where no queen
hangs.

Three reinforcements, each a thread already here:

- **Coverage** — the Zipf sliver inverted. Expert play concentrates on
  the well-trodden manifold and leaves the catastrophic regions
  unsampled; those are the regions a learner most needs. Random play
  splatters across the whole space, densely sampling the disasters.
  (AlphaZero starts random for exactly this — it learns the basics by
  watching queens hang and lose.)
- **Counterfactual / ablation** — Thorp, for free. Expert play
  suppresses the counterfactual (the master always defends, so you
  cannot tell whether defending mattered); random play runs the
  ablation, not defending and recording the result — a natural
  experiment, the road not taken tried a million times.
- **Outcome spectrum** — W needs crushing losses, not just the
  draws-and-small-edges experts over-represent; random play supplies
  the full distribution to calibrate against.

The nuance that completes it: a **curriculum, not a replacement.** Bad
examples teach the coarse structure first (the cliffs, what-not-to-do);
strong play teaches the fine structure later (technique, deep plans a
random walk never reaches). Both have coverage gaps — random rarely
reaches a subtle middlegame, experts never reach catastrophe — so they
are complementary, and the natural order is bad-first. That is not a
concession to the common-knowledge view; it is why that view is wrong
about *first*.

Build payoff: the v0.9 random corpus is the foundation layer, not a
placeholder. Folding it into the dictionary calibrates W in the
catastrophic regions where the boundary lives; the annotator pointed at
random games is a fumble generator — bad moves with labeled
consequences, the raw material for the case base of what-not-to-do.

**No maze: the position is the Markov state.** The objection to check
(Rob): many roads reach the same board, and forward probabilities must
not depend on the road. By construction they don't — the dictionary
keys on the value, never the route; the log lives in `Game`, and the
dictionary never sees it. The precise claim: `Position` carries
*exactly* the path residue that changes the future (castling rights, a
live en-passant window) and nothing else — the Markov property: given
the position, the future is independent of the path. The one piece of
counterfeit residue, a dead en-passant square no pawn can use, is
laundered by `repetition_key` — which is why the dictionary keys on
it. The king's-pawn and knight-first roads into the same opening
differ structurally by exactly that residue, and merge to one entry
(`dictionary_test.rs`). The honest margin: FIDE itself breaks the
Markov property at the edges — repetition counts and the fifty-move
clock make a few futures depend on history — and that dependence lives
at `Game` level, above the dictionary, exactly where the architecture
already put those rules.

### 5. The coach — the destination

The founding grievance (Rob): first game of computer chess in the
1980s, still remembers the first win against it — and in forty years,
no chess program has ever *coached*. They analyze ("eval -3.2, best
was Nf3"); they referee; they crush. None of them know the player.

The diagnosis: coaching requires two things engines structurally lack.
A **model of the student**, and a **vocabulary between centipawns and
grandmaster prose**. The other horizons turn out to be the missing
parts — the shift series most of all: a coach reviews the three plies
where W jumped, not all forty, and explains them in the key-family
vocabulary:

- *The log is the student's record.* Every game a replayable value;
  the coach was there for all of them and can return to any ply.
- *Think-time is the hesitation signal.* Where the clock burned is
  where the patterns ran out — a map of missing knowledge no app has
  ever read. A coach reads hesitation the way a human coach reads a
  face.
- *The key family is the diagnosis vocabulary.* "You lose in this pawn
  skeleton" — plan-level diagnosis in the quotient spaces, where
  engines can only mumble move-level centipawns.
- *The phrase book is the lesson unit.* Humans learn chess in chunks;
  that is why the chunks have names. Curriculum is phrases, ordered.
- *The player is the sparring partner* — tuned to play **into** the
  student's diagnosed weakness, which a maximally-strong engine never
  does. Losing instructively is a feature no engine has.

The cybernetic frame makes it precise (Wiener, via the `analog` crate
this vocabulary comes from): a coach is a feedback controller, and
effective control requires an analog of the controlled system inside
the controller — the coach must *contain a model of the student*: a
distribution over phrases they know, structures they understand,
hesitations they exhibit, updated after every game. And a lesson is a
long-running transaction with a re-verified post-condition: "the
king-safety leak is fixed" commits only if it is still true N games
later. Spaced repetition is ACID with continuously re-checked
invariants.

The measurable skeleton: diagnose (cluster the student's losses by
skeleton trajectory; spike-detect hesitation in their timelines),
prescribe (lessons from the phrase book; classics chosen because they
exhibit the missing plan — the Opera Game is already in the crate),
spar (an opponent tuned to steer into the weak structures), and
re-verify (the post-condition watched across future games). Every step
is a fold over logs.

### 6. Generalizing the game — extract, don't anticipate

The question (Rob): should `Game` become a trait, with chess merely an
implementation — and is a study branch then just a counterfactual?

**The counterfactual is the real insight, and it is bigger than "chess
is a game."** A study branch, an MCTS node, a Thorp ablation's
road-not-taken, an analysis variation — all one operation: a
counterfactual continuation from a state. The study tree, the search
tree, and the analysis tree are *one structure viewed three ways*, and
value semantics is what makes it free — a counterfactual is "keep the
old value and explore." `study.with(line)` grafts a real line, MCTS
expansion grafts a hypothetical one, ablation grafts the road not taken
and re-simulates: one verb, three uses. That unification, not "chess is
a game," is what would eventually justify an abstraction.

**But abstract by extraction, not anticipation.** caissa is good
*because* it was built concretely: chess pushed back and produced the
fallible reducer, Mode-derived-not-stored, `repetition_key` as a domain
equivalence, en passant as the third `Edit` verb, the Markov boundary.
None of those would have surfaced from a guessed `trait Game`. The
cautionary tale is in the same workspace — `effect-reducers`, the crate
this one rejected on day one: an abstraction designed before its
domain, with an infallible `reduce` that did not fit chess. A trait
earns its existence when two honest implementations share it (a second
game, or the first generic consumer), never before.

**And the boundary is not "Game trait / chess impl" — it is the
horizons.** Search, learning, and play are genuinely game-agnostic:
AlphaZero is one architecture over chess, shogi, and go. *Those* want a
trait; the chess crate implements it. The trait wants the **infallible**
half — `actions()` plus a total `apply` — because search only ever
applies moves it already enumerated as legal:

```rust
trait Rules {
    type State;
    type Action;
    type Outcome;
    fn actions(state: &Self::State) -> Vec<Self::Action>;       // legal continuations
    fn apply(state: &Self::State, action: &Self::Action) -> Self::State;  // total on legal
    fn outcome(state: &Self::State) -> Option<Self::Outcome>;   // None = ongoing
}
```

This resolves the very first argument of the project. The
fallible-vs-infallible reducer was never one question: `play()` is
fallible because *humans* send illegal moves (validation at the door),
and the search interface is infallible because it only explores the
legal. Two layers, not one — and caissa already has both halves
(`legal_actions` is the enumerator, `apply` the total evaluator; `play`
is the human-facing sugar). `effect-reducers`' infallible reducer was
not wrong; it was in the wrong layer.

So: build `study` concretely now. Extract `Rules` when the player
horizon arrives (it is the first consumer that genuinely needs the
generic interface) or when a second game joins to keep the trait
honest. The counterfactual tree is the structure study and search will
turn out to share — design study so that shape reads clearly, but do
not contort it toward a trait that has not yet earned two
implementations.

## Sequencing

Near-term (the checklist): v0.7.0 interchange (FEN + tagged PGN
export), then variations — where prefix sharing becomes API — then the
book staging, while the decisions are fresh. The horizons come after,
as layers and siblings: player, then learner, then the encoding lab —
each one a component the coach needs, so the destination orders the
road. The book gets chapters out of every stage; the tags mark where
each one stands.
