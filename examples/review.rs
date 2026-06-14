//! Annotate a game and find where it turned.
//!
//!     cargo run --example review
//!
//! The shift series prints each move with how far it swung the position
//! for the player who made it; the turning point is the worst swing. A
//! one-ply tactical eval is enough to see Fool's Mate; deeper play (and
//! famous sacrifices) would need a deeper eval — the verdict is only as
//! sharp as the estimator.

use caissa::classics::fools_mate;
use caissa::play::{evaluate, material};
use caissa::review::annotate;
use caissa::to_san;

fn main() {
    let game = fools_mate();
    println!("{}\n", game.score());

    let series = annotate(&game, &|p| evaluate(p, 1, &material));
    for a in &series {
        let san = to_san(game[a.ply - 1], a.action).unwrap();
        println!("  ply {:>2}  {:<5}  {:<6}  swing {:+}", a.ply, format!("{:?}", a.mover), san, a.swing);
    }

    let worst = series.iter().min_by_key(|a| a.swing).unwrap();
    let san = to_san(game[worst.ply - 1], worst.action).unwrap();
    println!("\n  turning point: move {} ({:?}'s {san}), swing {:+}", worst.ply, worst.mover, worst.swing);
}
