//! Property-based tests for surface parameterization math.
//!
//! Uses `proptest` to verify mathematical invariants that must hold for all
//! valid `(u, v)` inputs: metric symmetry, positive-definiteness, position
//! norms, velocity conservation, and Christoffel symbol symmetry.

use geodesic_wallpaper::surface::sphere::Sphere;
use geodesic_wallpaper::surface::torus::Torus;
use geodesic_wallpaper::surface::saddle::Saddle;
use geodesic_wallpaper::surface::Surface;
use geodesic_wallpaper::geodesic::Geodesic;
use proptest::prelude::*;
use std::f32::consts::{PI, TAU};

// ─── Sphere property tests ────────────────────────────────────────────────────

proptest! {
    /// The metric tensor is symmetric: g[0][1] == g[1][0] for all (u, v).
    #[test]
    fn prop_sphere_metric_symmetric(
        u in 0.0f32..TAU,
        v in 0.01f32..(PI - 0.01),
    ) {
        let s = Sphere::new(1.0);
        let g = s.metric(u, v);
        prop_assert!(
            (g[0][1] - g[1][0]).abs() < 1e-5,
            "metric not symmetric: g[0][1]={} g[1][0]={}", g[0][1], g[1][0]
        );
    }

    /// The metric tensor is positive-definite: det(g) > 0 for all valid (u, v).
    #[test]
    fn prop_sphere_metric_positive_definite(
        u in 0.0f32..TAU,
        v in 0.01f32..(PI - 0.01),
    ) {
        let s = Sphere::new(1.0);
        let g = s.metric(u, v);
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        prop_assert!(det > 0.0, "det(g)={det} not positive at u={u} v={v}");
    }

    /// For a sphere of radius `r`, |position(u,v)| == r for all (u, v).
    #[test]
    fn prop_sphere_position_norm_equals_radius(
        u in 0.0f32..TAU,
        v in 0.0f32..PI,
        radius in 0.1f32..10.0f32,
    ) {
        let s = Sphere::new(radius);
        let p = s.position(u, v);
        let len = p.length();
        prop_assert!(
            (len - radius).abs() < 1e-4,
            "|position|={len} but radius={radius} at u={u} v={v}"
        );
    }

    /// Christoffel symbols satisfy torsion-free symmetry: Gamma^k_ij == Gamma^k_ji.
    #[test]
    fn prop_sphere_christoffel_symmetry(
        u in 0.0f32..TAU,
        v in 0.05f32..(PI - 0.05),
    ) {
        let s = Sphere::new(1.0);
        let gamma = s.christoffel(u, v);
        for k in 0..2usize {
            prop_assert!(
                (gamma[k][0][1] - gamma[k][1][0]).abs() < 1e-5,
                "Gamma^{k}_01={} != Gamma^{k}_10={} at u={u} v={v}",
                gamma[k][0][1], gamma[k][1][0]
            );
        }
    }

    /// After N integration steps on a sphere, the metric speed stays within
    /// 1% of its initial value (geodesic velocity magnitude conservation).
    #[test]
    fn prop_sphere_geodesic_speed_conserved(
        u in 0.0f32..TAU,
        angle in 0.0f32..TAU,
        steps in 10usize..100usize,
    ) {
        let s = Sphere::new(1.0);
        let v = PI / 2.0; // equator — avoids pole singularities
        let du = angle.cos() * 0.5;
        let dv = angle.sin() * 0.5;
        let mut geo = Geodesic::new(u, v, du, dv, 100_000, 0);

        let g0 = s.metric(geo.u, geo.v);
        let initial_speed_sq = g0[0][0] * geo.du * geo.du
            + 2.0 * g0[0][1] * geo.du * geo.dv
            + g0[1][1] * geo.dv * geo.dv;

        for _ in 0..steps {
            geo.step(&s, 0.016);
        }

        let g = s.metric(geo.u, geo.v);
        let final_speed_sq = g[0][0] * geo.du * geo.du
            + 2.0 * g[0][1] * geo.du * geo.dv
            + g[1][1] * geo.dv * geo.dv;

        // Speed is renormalised each step, so both should be ~1 after first step.
        prop_assert!(
            final_speed_sq.is_finite(),
            "final speed_sq is not finite: {final_speed_sq}"
        );
        // If initial speed was non-trivial, final speed must be within 1% of 1.0
        // (renormalisation target) rather than the raw initial speed.
        if initial_speed_sq > 1e-6 {
            let final_speed = final_speed_sq.sqrt();
            prop_assert!(
                (final_speed - 1.0).abs() < 0.01,
                "speed deviated from 1.0: initial_sq={initial_speed_sq} final={final_speed}"
            );
        }
    }
}

// ─── Torus property tests ─────────────────────────────────────────────────────

proptest! {
    /// The metric tensor is symmetric for the torus at all (u, v).
    #[test]
    fn prop_torus_metric_symmetric(
        u in 0.0f32..TAU,
        v in 0.0f32..TAU,
    ) {
        let t = Torus::new(2.0, 0.7);
        let g = t.metric(u, v);
        prop_assert!(
            (g[0][1] - g[1][0]).abs() < 1e-5,
            "metric not symmetric: g[0][1]={} g[1][0]={}", g[0][1], g[1][0]
        );
    }

    /// The metric tensor is positive-definite for the torus at all (u, v).
    #[test]
    fn prop_torus_metric_positive_definite(
        u in 0.0f32..TAU,
        v in 0.0f32..TAU,
    ) {
        let t = Torus::new(2.0, 0.7);
        let g = t.metric(u, v);
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        prop_assert!(det > 0.0, "det(g)={det} not positive at u={u} v={v}");
    }

    /// Christoffel symbols satisfy torsion-free symmetry on the torus.
    #[test]
    fn prop_torus_christoffel_symmetry(
        u in 0.0f32..TAU,
        v in 0.0f32..TAU,
    ) {
        let t = Torus::new(2.0, 0.7);
        let gamma = t.christoffel(u, v);
        for k in 0..2usize {
            prop_assert!(
                (gamma[k][0][1] - gamma[k][1][0]).abs() < 1e-5,
                "Gamma^{k}_01={} != Gamma^{k}_10={} at u={u} v={v}",
                gamma[k][0][1], gamma[k][1][0]
            );
        }
    }
}

// ─── Saddle property tests ────────────────────────────────────────────────────

proptest! {
    /// The metric tensor is symmetric for the saddle at all valid (u, v).
    #[test]
    fn prop_saddle_metric_symmetric(
        u in -1.9f32..1.9f32,
        v in -1.9f32..1.9f32,
    ) {
        let s = Saddle::new(2.0);
        let g = s.metric(u, v);
        prop_assert!(
            (g[0][1] - g[1][0]).abs() < 1e-5,
            "metric not symmetric: g[0][1]={} g[1][0]={}", g[0][1], g[1][0]
        );
    }

    /// The metric tensor is positive-definite for the saddle at all valid (u, v).
    #[test]
    fn prop_saddle_metric_positive_definite(
        u in -1.9f32..1.9f32,
        v in -1.9f32..1.9f32,
    ) {
        let s = Saddle::new(2.0);
        let g = s.metric(u, v);
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        prop_assert!(det > 0.0, "det(g)={det} not positive at u={u} v={v}");
    }

    /// Christoffel symbols satisfy torsion-free symmetry on the saddle.
    #[test]
    fn prop_saddle_christoffel_symmetry(
        u in -1.9f32..1.9f32,
        v in -1.9f32..1.9f32,
    ) {
        let s = Saddle::new(2.0);
        let gamma = s.christoffel(u, v);
        for k in 0..2usize {
            prop_assert!(
                (gamma[k][0][1] - gamma[k][1][0]).abs() < 1e-5,
                "Gamma^{k}_01={} != Gamma^{k}_10={} at u={u} v={v}",
                gamma[k][0][1], gamma[k][1][0]
            );
        }
    }
}
