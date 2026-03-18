//! Integration tests for the geodesic integrator.

use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::surface::{torus::Torus, sphere::Sphere, Surface};
use std::f32::consts::PI;

#[test]
fn geodesic_new_initialises_correctly() {
    let g = Geodesic::new(0.1, 0.2, 1.0, 0.5, 300, 2);
    assert!(g.alive);
    assert_eq!(g.age, 0);
    assert_eq!(g.max_age, 300);
    assert_eq!(g.color_idx, 2);
    assert!((g.u - 0.1).abs() < 1e-6);
    assert!((g.v - 0.2).abs() < 1e-6);
}

#[test]
fn geodesic_step_advances_position() {
    let torus = Torus::new(2.0, 0.7);
    let mut g = Geodesic::new(0.0, 0.0, 1.0, 0.0, 1000, 0);
    let u0 = g.u;
    let v0 = g.v;
    g.step(&torus, 0.016);
    // After one step, at least one coordinate should have changed.
    assert!(
        (g.u - u0).abs() > 1e-7 || (g.v - v0).abs() > 1e-7,
        "position did not advance after one step"
    );
}

#[test]
fn geodesic_age_increments_per_step() {
    let torus = Torus::new(2.0, 0.7);
    let mut g = Geodesic::new(0.5, 0.5, 0.3, 0.2, 1000, 0);
    assert_eq!(g.age, 0);
    g.step(&torus, 0.016);
    assert_eq!(g.age, 1);
    g.step(&torus, 0.016);
    assert_eq!(g.age, 2);
}

#[test]
fn geodesic_dies_at_max_age() {
    let sphere = Sphere::new(1.0);
    let mut g = Geodesic::new(0.0, PI / 2.0, 0.5, 0.0, 3, 0);
    for i in 0..3 {
        assert!(g.alive, "should be alive at step {i}");
        g.step(&sphere, 0.016);
    }
    assert!(!g.alive, "should be dead after max_age steps");
}

#[test]
fn geodesic_position_and_velocity_remain_finite() {
    let torus = Torus::new(2.0, 0.7);
    let mut g = Geodesic::new(0.5, 0.5, 0.3, 0.2, 10_000, 0);
    for _ in 0..200 {
        g.step(&torus, 0.016);
    }
    assert!(g.u.is_finite(), "u is not finite");
    assert!(g.v.is_finite(), "v is not finite");
    assert!(g.du.is_finite(), "du is not finite");
    assert!(g.dv.is_finite(), "dv is not finite");
}

#[test]
fn geodesic_no_nan_after_many_steps_on_sphere() {
    let sphere = Sphere::new(1.0);
    let mut g = Geodesic::new(0.0, PI / 2.0, 0.5, 0.0, 50_000, 0);
    for _ in 0..500 {
        g.step(&sphere, 0.016);
    }
    assert!(!g.u.is_nan(), "u is NaN");
    assert!(!g.v.is_nan(), "v is NaN");
    assert!(!g.du.is_nan(), "du is NaN");
    assert!(!g.dv.is_nan(), "dv is NaN");
}

#[test]
fn geodesic_metric_speed_conserved_on_torus() {
    let torus = Torus::new(2.0, 0.7);
    let mut g = Geodesic::new(0.3, 0.5, 0.5, 0.3, 10_000, 0);
    let dt = 0.016_f32;
    for _ in 0..500 {
        g.step(&torus, dt);
    }
    let m = torus.metric(g.u, g.v);
    let speed_sq = m[0][0] * g.du * g.du + 2.0 * m[0][1] * g.du * g.dv + m[1][1] * g.dv * g.dv;
    assert!(
        (speed_sq.sqrt() - 1.0).abs() < 0.02,
        "metric speed deviated from 1: {}",
        speed_sq.sqrt()
    );
}
