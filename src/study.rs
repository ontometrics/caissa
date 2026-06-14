//! Variations: a study is a tree of lines sharing prefixes.
//!
//! The first namespaced subsystem — `caissa::study`, opt-in like
//! `notation`/`pgn`/`classics`, while the core stays flat at the root.
//! A [`Study`] is a layer above [`Game`](crate::Game): the game stays a
//! linear fold, and branching is added without touching a single law it
//! already proves.
//!
//! You build a study by *grafting whole lines* — never by addressing
//! nodes. The tree walks each new line from the root, coincides along
//! the shared prefix, and branches where it diverges, so prefix sharing
//! is the construction mechanism, not merely the storage: the vision's
//! "a tree of logs sharing prefixes" is literal here (a node holds an
//! [`Action`] — the truth — and its continuations; positions are derived
//! by folding, never stored). The first line grafted is the mainline;
//! child 0 is the mainline continuation at every node, recursively, so a
//! variation of a variation needs no new concept.
//!
//! ```
//! use caissa::classics::ruy_lopez;
//! use caissa::study::Study;
//!
//! let mainline = ruy_lopez();                      // …Nc6 3. Bb5
//! let variation = mainline.undo().apply("Bc4")?;   // …Nc6 3. Bc4 instead
//! let study = Study::from(mainline).with(variation);
//! // two lines, sharing their first four plies
//! # let _ = study.mainline();
//! # Ok::<(), caissa::Rejected>(())
//! ```

use crate::action::Action;
use crate::game::Game;
use crate::position::Position;

/// A tree of lines from one starting position. Build it by grafting
/// games with [`with`](Study::with); read it back as games with
/// [`mainline`](Study::mainline) and [`lines`](Study::lines).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Study {
    start: Position,
    branches: Vec<Node>,
}

/// One move in the tree: the action, and the continuations that follow
/// it. `branches[0]` is the mainline continuation; the rest are
/// variations. The position is *not* stored — it is the fold of the
/// actions from the start, derived on demand.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Node {
    action: Action,
    branches: Vec<Node>,
}

impl Study {
    /// An empty study from the standard starting position — one trivial
    /// line, no moves.
    pub fn new() -> Study {
        Study::from_position(Position::default())
    }

    /// An empty study from a given start.
    pub fn from_position(start: Position) -> Study {
        Study { start, branches: Vec::new() }
    }

    /// A game as a study of a single line.
    pub fn from(game: Game) -> Study {
        let start = game[0];
        Study::from_position(start).with(game)
    }

    /// Graft a line onto the study. The tree merges it along the shared
    /// prefix and branches where it diverges; re-grafting an existing
    /// line changes nothing. The first line grafted defines the
    /// mainline. Lines must share the study's start — automatic when you
    /// build them from the mainline (`undo`/`apply`) or one PGN.
    pub fn with(mut self, line: Game) -> Study {
        debug_assert!(line[0] == self.start, "a grafted line must share the study's start");
        graft(&mut self.branches, line.log());
        self
    }

    /// The mainline as a game — child 0 followed to the end.
    pub fn mainline(&self) -> Game {
        let mut actions = Vec::new();
        let mut branches = &self.branches;
        while let Some(node) = branches.first() {
            actions.push(node.action);
            branches = &node.branches;
        }
        self.game_of(&actions)
    }

    /// Every line in the study as a game, mainline first — each carrying
    /// all of `Game`'s derived queries (`mode`, `score`, `fen`, …).
    pub fn lines(&self) -> impl Iterator<Item = Game> {
        let mut out = Vec::new();
        self.collect(&self.branches, &mut Vec::new(), &mut out);
        out.into_iter()
    }

    fn collect(&self, branches: &[Node], path: &mut Vec<Action>, out: &mut Vec<Game>) {
        if branches.is_empty() {
            out.push(self.game_of(path));
            return;
        }
        for node in branches {
            path.push(node.action);
            self.collect(&node.branches, path, out);
            path.pop();
        }
    }

    /// Fold a path of actions into a game from the study's start. The
    /// actions came from grafted games, so they always replay.
    fn game_of(&self, actions: &[Action]) -> Game {
        actions.iter().fold(Game::from_position(self.start), |game, &action| {
            game.apply(action).expect("a study holds only accepted actions")
        })
    }
}

impl Default for Study {
    fn default() -> Study {
        Study::new()
    }
}

/// Walk `actions` into `branches`, descending shared moves and creating a
/// fresh chain where the line first diverges. Index-then-borrow keeps the
/// borrow checker happy (no overlapping mutable borrow of `branches`).
fn graft(branches: &mut Vec<Node>, actions: &[Action]) {
    let Some((first, rest)) = actions.split_first() else {
        return;
    };
    match branches.iter().position(|node| node.action == *first) {
        Some(existing) => graft(&mut branches[existing].branches, rest),
        None => {
            let mut node = Node { action: *first, branches: Vec::new() };
            graft(&mut node.branches, rest);
            branches.push(node);
        }
    }
}
