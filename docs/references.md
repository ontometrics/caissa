# References

Works the vision draws on or answers — papers first, then the
conceptual touchstones named in [vision.md](vision.md). Each line says
why it is here, so the allusions in the vision have a source.

## Papers

- **Recursive Language Models** — Zhang, Kraska, Khattab, 2025.
  [arXiv:2512.24601](https://arxiv.org/abs/2512.24601). Decompose a
  too-long prompt and recurse, recursion as inference-time scaling.
  Relevance: the engine already *is* structural recursion (search, the
  trees); the fit is the sibling layer — a recursive interpreter reading
  a whole career, with caissa supplying the decomposition (the study
  tree, phases, key-family clusters, fumble-cases) instead of blind
  token windows. See "Recursive interpreters" in the vision.

- **Can Large Language Models Develop Strategic Reasoning?
  Post-training Insights from Learning Chess** — Hwang, Lee, Choo, Park,
  Park, 2025. [arXiv:2507.00726](https://arxiv.org/abs/2507.00726).
  RL-trains chess LLMs with dense reward from a chess action-value
  network (distillation); dense beats sparse but plateaus below expert,
  the ceiling being the base model's chess understanding, which RL
  cannot supply. Relevance: corroborates two threads — dense reward *is*
  credit assignment (the annotator's shift series), and the plateau *is*
  the feature/encoding gap W − Ŵ (why "encode attack-forms" matters; RL
  cannot manufacture understanding the representation has no features
  for).

- **A general reinforcement learning algorithm that masters chess,
  shogi and Go through self-play (AlphaZero)** — Silver et al., Science,
  2018 (preprint [arXiv:1712.01815](https://arxiv.org/abs/1712.01815)).
  Self-play + MCTS + a value/policy network, one architecture over three
  games. Relevance: the player/learn horizons and the council (search
  blended with a learned policy/eval); the "generalize, don't
  anticipate" boundary (the trait lives at the game-agnostic layer);
  self-play starts random — bad games teach the boundary.

- **Neural Machine Translation of Rare Words with Subword Units (BPE)**
  — Sennrich, Haddow, Birch, 2016.
  [arXiv:1508.07909](https://arxiv.org/abs/1508.07909). Byte-pair
  encoding discovers frequent subsequences and makes each a token.
  Relevance: the phrase-book conjecture — BPE over Edit/SAN streams
  should rediscover chess's named idioms; the encoding laboratory.

## Conceptual touchstones

- **Peirce, "The Fixation of Belief"** (1877). The *a priori* method —
  belief fixed by what is agreeable to reason rather than tested — names
  the "common-knowledge pit" (e.g. "random games are useless data,"
  which is backwards: bad games teach the boundary).
- **Shannon, "Programming a Computer for Playing Chess"** (1950), and
  **"A Mathematical Theory of Communication"** (1948). The game tree and
  evaluation; and the n-gram / frequency model the "most common move"
  table is (the pre-LLM answer to play, the true job of which is the
  opponent model).
- **Bellman, "Dynamic Programming"** (1957). The principle of optimality
  — greedy on the true value is optimal; minimax is the fixed point to a
  horizon. The Mencken-trap resolution.
- **Thorp, "Beat the Dealer"** (1962). Value from counterfactuals at
  scale — perturb, simulate, measure. The shift series and ablation;
  random self-play as a free natural experiment.
- **Kelly, "A New Interpretation of Information Rate"** (1956). Bet to
  match your edge, never risk ruin. Risk appetite as a function of W
  (behind → seek variance; ahead → kill it; the absorbing barrier).
- **Wiener, "Cybernetics"** (1948). Effective control needs an analog of
  the system inside the controller. The coach contains a model of the
  student; competence-gating in the council. (Also the `analog` crate.)
- **Snodgrass, bitemporal / valid-time** (e.g. *Developing Time-Oriented
  Database Applications in SQL*, 1999). The `started`/`ended` interval
  in `Timeline`; the repertoire's evolution over time.
- **Epictetus, *Enchiridion*** (~125 AD). The dichotomy of control —
  your moves vs the circumstance you are handed — gives the repertoire
  tree its asymmetric shape (narrow at your nodes, wide at the
  opponent's).
- **Mencken**: "For every complex problem there is an answer that is
  clear, simple, and wrong." The greedy-on-Ŵ trap.
- **Lewis, *Moneyball*** (2003). The market reprices when better
  features arrive (Steinitz over the Romantics); brilliance is a market
  inefficiency arbitraged away; the gap is feature-shaped.
