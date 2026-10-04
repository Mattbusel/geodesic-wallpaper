//! Follow one geodesic around a torus and print where it goes.
//!
//! Run with `cargo run --example follow_geodesic`.

use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::surface::{torus::Torus, Surface};

fn main() {
    let torus = Torus::new(2.0, 0.7);
    // Start at (u, v) = (0, 0) heading in direction (1, 0.3); live 300 steps.
    let mut g = Geodesic::new(0.0, 0.0, 1.0, 0.3, 300, 0);
    for step in 0..=100 {
        if step % 25 == 0 {
            let p = torus.position(g.u, g.v);
            println!("step {step:3}: ({:+.3}, {:+.3}, {:+.3})", p.x, p.y, p.z);
        }
        g.step(&torus, 0.016); // one RK4 step of the geodesic equation
    }
}
