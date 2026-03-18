//! Integration tests for `surface::Torus`, `surface::Sphere`, and `surface::Saddle`.

use geodesic_wallpaper::surface::{torus::Torus, sphere::Sphere, saddle::Saddle, Surface};
use std::f32::consts::{PI, TAU};

// ---- Torus ---------------------------------------------------------------

#[test]
fn torus_position_at_u0_v0() {
    let t = Torus::new(2.0, 0.7);
    let p = t.position(0.0, 0.0);
    // At u=0,v=0: x=R+r=2.7, y=0, z=0.
    assert!((p.x - 2.7).abs() < 1e-5, "x={}", p.x);
    assert!(p.y.abs() < 1e-5, "y={}", p.y);
    assert!(p.z.abs() < 1e-5, "z={}", p.z);
}

#[test]
fn torus_position_magnitude_reasonable() {
    let t = Torus::new(2.0, 0.7);
    for (u, v) in [(0.0f32, 0.0f32), (1.0, 1.0), (2.5, 3.0)] {
        let p = t.position(u, v);
        let len = p.length();
        // Should be between R-r=1.3 and R+r=2.7 (projected onto XY plane, z small).
        assert!(len >= 1.2 && len <= 3.0, "position magnitude {len} out of range at ({u},{v})");
    }
}

#[test]
fn torus_metric_positive_definite() {
    let t = Torus::new(2.0, 0.7);
    for ui in 0..4u32 {
        for vi in 0..4u32 {
            let u = ui as f32 * TAU / 4.0;
            let v = vi as f32 * TAU / 4.0;
            let g = t.metric(u, v);
            assert!(g[0][0] > 0.0, "g_00 not positive at ({u},{v})");
            assert!(g[1][1] > 0.0, "g_11 not positive at ({u},{v})");
            let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
            assert!(det > 0.0, "det(g) not positive at ({u},{v})");
        }
    }
}

#[test]
fn torus_christoffel_finite() {
    let t = Torus::new(2.0, 0.7);
    let gamma = t.christoffel(0.5, 1.0);
    for k in 0..2usize {
        for i in 0..2usize {
            for j in 0..2usize {
                assert!(gamma[k][i][j].is_finite(), "Γ^{k}_{i}{j} is not finite");
            }
        }
    }
}

#[test]
fn torus_christoffel_symmetric_lower() {
    let t = Torus::new(2.0, 0.7);
    let g = t.christoffel(0.4, 0.8);
    for k in 0..2 {
        assert!(
            (g[k][0][1] - g[k][1][0]).abs() < 1e-6,
            "Γ^{k}_01 != Γ^{k}_10"
        );
    }
}

// ---- Sphere --------------------------------------------------------------

#[test]
fn sphere_position_on_unit_sphere() {
    let s = Sphere::new(1.0);
    for (u, v) in [(0.0f32, PI / 2.0), (1.0, 1.0), (3.0, 2.0)] {
        let len = s.position(u, v).length();
        assert!((len - 1.0).abs() < 1e-5, "sphere position len={len} at ({u},{v})");
    }
}

#[test]
fn sphere_normal_perpendicular_to_tangents() {
    let s = Sphere::new(1.5);
    for (u, v) in [(0.5f32, 1.0f32), (2.0, 0.8)] {
        let pos = s.position(u, v);
        let n = s.normal(u, v);
        // For a sphere, normal == normalised position.
        let dot = pos.normalize().dot(n);
        assert!((dot - 1.0).abs() < 1e-5, "normal not radial at ({u},{v}): dot={dot}");
    }
}

#[test]
fn sphere_metric_positive_definite() {
    let s = Sphere::new(2.0);
    for ui in 0..4u32 {
        let u = ui as f32 * TAU / 4.0;
        let v = 0.5 + ui as f32 * 0.3; // keep away from poles
        let g = s.metric(u, v);
        assert!(g[0][0] > 0.0, "g_00 not positive");
        assert!(g[1][1] > 0.0, "g_11 not positive");
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        assert!(det > 0.0, "det(g) not positive");
    }
}

#[test]
fn sphere_christoffel_finite_away_from_poles() {
    let s = Sphere::new(1.0);
    let gamma = s.christoffel(0.3, PI / 2.0);
    for k in 0..2usize {
        for i in 0..2usize {
            for j in 0..2usize {
                assert!(gamma[k][i][j].is_finite(), "Γ^{k}_{i}{j} not finite");
            }
        }
    }
}

// ---- Saddle --------------------------------------------------------------

#[test]
fn saddle_position_at_origin() {
    let s = Saddle::new(2.0);
    let p = s.position(0.0, 0.0);
    assert!(p.x.abs() < 1e-6 && p.y.abs() < 1e-6 && p.z.abs() < 1e-6);
}

#[test]
fn saddle_metric_positive_definite() {
    let s = Saddle::new(2.0);
    for (u, v) in [(0.0f32, 0.0f32), (0.5, 0.5), (-1.0, 1.0)] {
        let g = s.metric(u, v);
        assert!(g[0][0] > 0.0, "g_00 not positive at ({u},{v})");
        assert!(g[1][1] > 0.0, "g_11 not positive at ({u},{v})");
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        assert!(det > 0.0, "det(g) not positive at ({u},{v})");
    }
}

#[test]
fn saddle_christoffel_finite() {
    let s = Saddle::new(2.0);
    let gamma = s.christoffel(0.5, 0.5);
    for k in 0..2usize {
        for i in 0..2usize {
            for j in 0..2usize {
                assert!(gamma[k][i][j].is_finite(), "Γ^{k}_{i}{j} not finite");
            }
        }
    }
}

#[test]
fn saddle_christoffel_at_origin_zero() {
    let s = Saddle::new(2.0);
    let gamma = s.christoffel(0.0, 0.0);
    for k in 0..2usize {
        for i in 0..2usize {
            for j in 0..2usize {
                assert!(gamma[k][i][j].abs() < 1e-5, "Γ^{k}_{i}{j} != 0 at origin");
            }
        }
    }
}
