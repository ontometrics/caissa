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

A parsed description is just constraints:

```rust
struct Description {
    role: Role,
    to: Square,
    from_file: Option<u8>,   // the "b" in Nbd2
    from_rank: Option<u8>,   // the "1" in R1e2
    promotion: Option<Role>, // the "=Q"
    // capture "x", check "+", mate "#" are validated but not needed to resolve
}
```

`O-O` / `O-O-O` skip description entirely: they desugar directly to the
king's two-square move for the side to move. Castling already has no
notation in the core; this is the import-time sugar we promised.

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
