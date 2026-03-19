//! Long-running stability soak tests.
//!
//! These tests simulate 10,000 frames with 30 geodesics each and assert that
//! no NaN or infinity values appear in the surface positions.
//!
//! Run with:
//! ```text
//! cargo test --test soak -- --ignored
//! ```

use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::surface::Surface;
use geodesic_wallpaper::surface::{
    catenoid::Catenoid, helicoid::Helicoid, hyperboloid::Hyperboloid, saddle::Saddle,
    sphere::Sphere, torus::Torus,
};
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

fn run_soak<S: Surface + 'static>(surface: S, name: &str) {
    let mut rng = StdRng::seed_from_u64(42);
    let surface = Arc::new(surface);
    let dt = 0.016_f32;

    let mut geodesics: Vec<Geodesic> = (0..30)
        .map(|_| {
            let (u, v) = surface.random_position(&mut rng);
            let (du, dv) = surface.random_tangent(u, v, &mut rng);
            // Use a very large max_age so geodesics don't die during the soak.
            Geodesic::new(u, v, du, dv, 1_000_000, 0)
        })
        .collect();

    for frame in 0..10_000_usize {
        for geo in geodesics.iter_mut() {
            geo.step(&*surface, dt);
            let pos = surface.position(geo.u, geo.v);
            assert!(
                pos.x.is_finite() && pos.y.is_finite() && pos.z.is_finite(),
                "NaN/inf position on surface '{}' at frame {} (u={}, v={})",
                name,
                frame,
                geo.u,
                geo.v,
            );
            assert!(
                geo.u.is_finite() && geo.v.is_finite(),
                "NaN/inf parameter on surface '{}' at frame {} (u={}, v={})",
                name,
                frame,
                geo.u,
                geo.v,
            );
        }
    }

    println!(
        "Soak test passed for {} (10,000 frames, 30 geodesics)",
        name
    );
}

#[test]
#[ignore]
fn soak_torus() {
    run_soak(Torus::new(3.0, 1.0), "torus");
}

#[test]
#[ignore]
fn soak_sphere() {
    run_soak(Sphere::new(1.0), "sphere");
}

#[test]
#[ignore]
fn soak_saddle() {
    run_soak(Saddle::new(1.0), "saddle");
}

#[test]
#[ignore]
fn soak_catenoid() {
    run_soak(Catenoid::new(1.0), "catenoid");
}

#[test]
#[ignore]
fn soak_helicoid() {
    run_soak(Helicoid::new(1.0), "helicoid");
}

#[test]
#[ignore]
fn soak_hyperboloid() {
    run_soak(Hyperboloid::new(1.0, 1.0), "hyperboloid");
}
