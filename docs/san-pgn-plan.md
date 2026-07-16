# Plan: SAN and PGN import (v0.3.0)

The goal, stated as the test that ends the work: fold the Opera Game from
its PGN and print the final board.

```rust
let game = caissa::import(OPERA_GAME_PGN)?;
assert_that!(
    game.mode(),
    eq(Mode::Played(Ending::Checkmate { winner: Color::White }))
);
println!("{}", game[Terminus]);
```

## Decisions (the part worth writing down)

### 1. SAN is resolved, not interpreted

"Nf3" is a *description*, not an instruction. Resolution is:

    parse the description → filter `legal_actions()` by it → demand exactly one match

This buys total correctness for free: SAN inherits every legality rule
the reducer already enforces, including disambiguation ("Nbd2" when two
knights reach d2) and the rule that a SAN move matching zero legal
actions is illegal *for whatever reason the reducer would have given*.
Zero matches → rejected; two or more → `AmbiguousSan { candidates }`.
Errors stay data.

A parsed SAN is a sum type — every assertion a named state, no
option-soup. Its variants mirror `Action`'s (plus `Castle`, which
desugars):

```rust
/// What a SAN string asserts about the move it names.
enum San {
    Castle(Wing),                                     // O-O, O-O-O
    Move { role: Role, origin: Origin, to: Square },  // e4, Nf3, Nbd2, Rxe5
    Promote { origin: Origin, to: Square, into: Role }, // e8=Q, exd8=Q
}

/// Where the mover comes from — as much as the text cares to say.
enum Origin {
    Anywhere,       // Nf3
    File(u8),       // Nbd2 — the b-knight
    Rank(u8),       // R1e2 — the first-rank rook
    Square(Square), // Qh4e1 — fully spelled out
}
```

What the old `Option`-pile couldn't say, this says plainly: missingness
has a name (`Anywhere`), full-square disambiguation is a real state
instead of "both options coincidentally set", and promotion is its own
variant exactly as it is in `Action` — so `San::Move` resolves against
`Action::Move` candidates and `San::Promote` against `Action::Promote`,
with `Origin` as the filter on from-squares. Capture `x`, check `+`,
and mate `#` are validated during parse but never needed to resolve.

`O-O` / `O-O-O` desugar directly to the king's two-square move for the
side to move. Castling already has no notation in the core; this is the
import-time sugar we promised.

### 2. `IntoAction` becomes position-aware

Today `IntoAction::into_action(self)` cannot resolve SAN — "Nf3" needs
the position. The fix is to thread it through:

```rust
trait IntoAction {
    fn into_action(self, position: Position) -> Result<Action, Rejected>;
}
```

UCI strings and `(Square, Square)` ignore the argument; SAN uses it.
String sniffing: a token matching `[a-h][1-8][a-h][1-8][qrbn]?` is UCI,
anything else tries SAN. The payoff is the API from the very first
conversation about this crate:

```rust
game.apply("e4")?    // SAN pawn push — finally
game.apply("Nf3")?
game.apply("O-O")?
```

This is the one breaking change (trait signature), hence the minor bump.

## Stages

1. **Description parser + resolver** — `san.rs`. Parse SAN into
   `Description`, resolve against `legal_actions()`. New `Rejected`
   variants: `AmbiguousSan { candidates: Vec<Action> }` (and reuse
   `Unparseable`). Tests: pawn pushes, piece moves, captures,
   disambiguation by file/rank, promotion (`e8=Q`), castling both wings,
   a SAN that names an illegal move, a genuinely ambiguous SAN.
2. **Position-aware `IntoAction`** — thread `Position` through the trait,
   update the three existing impls, add the sniffing `&str` impl.
   Everything downstream (`play`, `+` stays `Action`-only, `Game::apply`,
   `Timeline::record`) keeps working; existing tests prove it.
3. **PGN parser** — `pgn.rs`, hand-rolled parser combinators (no deps;
   same idiom as baker's parser chapters). v0.3.0 scope: tag pairs into a
   map, movetext to SAN tokens, skip comments `{...}`, move numbers, and
   NAGs `$n`; reject variations `(...)` loudly rather than mis-parse.
   Termination marker (`1-0`, `0-1`, `1/2-1/2`, `*`) checked against the
   folded game's mode.

   > **Amended after the fact (Rob's review).** What shipped is a
   > hand-written *state machine*, not combinators — and the promise
   > above is voided, not deferred. The grammar this stage accepts is
   > regular (tag lines, a flat token stream, fixed-shape SAN), and a
   > state machine is the honest tool for a regular grammar. Even
   > variations don't revive the IOU: they add exactly *one* recursive
   > production (`variation = "(" sequence ")"`), which calls for one
   > recursive function grafted onto this tokenizer — recursive descent,
   > yes; the combinator idiom, no. Combinators earn their keep on
   > grammatical mass (many productions, alternation, reuse), and a
   > combinator library with a single call site is a framework for one
   > function. Match the parsing technology to the grammar's mass —
   > baker's grammar has it, PGN's doesn't. Extract, don't anticipate,
   > applied to parsers.
   >
   > **Refinement (Rob):** there *are* two move parsers — coordinate
   > (`e2e4`, `Action::FromStr`) and shorthand (`Nf5`, `San::FromStr`) —
   > and the sniffing `IntoAction for &str` alternates them:
   > `parse::<Action>().or_else(|_| parse::<San>()?.resolve(...))`. That
   > `or_else` *is* the `alt` combinator, stdlib-flavored, its ordering
   > justified by grammar disjointness; and the `FromStr` family
   > (`Square`, `Action`, `San`) is a set of parser values with a uniform
   > interface, composed by sequencing (`Action` and `San` both call
   > `Square`'s parser). So the crate has **combinator structure without
   > combinator machinery**. What a library adds — a generic parser type
   > with *remainder threading* (`(T, rest)`), `many`, `separated_list` —
   > is unneeded here because tokenization is separable: the PGN state
   > machine whitespace-splits movetext before the token parsers run, so
   > no parser ever reports where it stopped. The book lesson:
   > combinators are a structure latent in `FromStr + Result` (`or_else`
   > = alt, `?` = seq); the library is that structure reified when
   > composition count and remainder threading cross a threshold. Baker
   > crosses it; caissa sits just below it.
4. **`import`** — `pub fn import(pgn: &str) -> Result<Game, Rejected>`:
   the movetext folded over `Game::apply`. Victory-lap tests: the Opera
   Game (has O-O-O and mate) and the Immortal Game (wild captures and
   sacrifices), each asserting the final position and mode.
5. **Tag `v0.3.0`.**

## Explicitly deferred

- SAN *emission* (`Action → "Nf3"`): needed for PGN export and
  "did you mean Nf6?" errors. Its disambiguation logic is the mirror of
  resolution. v0.4 territory.
- Variations, comments-as-data, NAGs-as-data: parse-and-skip for now.
- FEN start positions in tags (`[SetUp "1"]` / `[FEN "..."]`):
  `Game::from_position` already exists, so this is cheap later.
