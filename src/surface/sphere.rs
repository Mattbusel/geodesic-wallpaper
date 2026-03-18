//! Sphere surface parameterization with analytic Christoffel symbols.

use super::Surface;
use glam::Vec3;
use std::f32::consts::{TAU, PI};

/// Sphere of given radius using the standard spherical parameterization.
///
/// Parameter domain: `u in [0, 2pi)` (longitude), `v in (0, pi)` (colatitude).
///
/// ```text
/// x = r sin(v) cos(u)
/// y = r sin(v) sin(u)
/// z = r cos(v)
/// ```
///
/// All geodesics on the sphere are great circles. The Christoffel symbols are
/// computed analytically from the metric `g_00 = r^2 sin^2(v)`, `g_11 = r^2`,
/// `g_01 = 0`.
pub struct Sphere {
    /// Radius of the sphere.
    pub radius: f32,
}

impl Sphere {
    /// Construct a sphere with the given `radius`.
    pub fn new(radius: f32) -> Self { Self { radius } }

    /// First partial derivative `∂φ/∂u` at `(u, v)`.
    fn d_du(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(-self.radius * v.sin() * u.sin(),
                   self.radius * v.sin() * u.cos(),
                   0.0)
    }

    /// First partial derivative `∂φ/∂v` at `(u, v)`.
    fn d_dv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new( self.radius * v.cos() * u.cos(),
                   self.radius * v.cos() * u.sin(),
                  -self.radius * v.sin())
    }
}

impl Surface for Sphere {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(self.radius * v.sin() * u.cos(),
                  self.radius * v.sin() * u.sin(),
                  self.radius * v.cos())
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, _u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // g_00 = r² sin²v, g_11 = r², g_01 = 0
        // Γ^0_01 = Γ^0_10 = cos(v)/sin(v)
        // Γ^1_00 = -sin(v)cos(v)
        let sv = v.sin();
        let cv = v.cos();
        let gamma_0_01 = if sv.abs() > 1e-6 { cv / sv } else { 0.0 };
        let gamma_1_00 = -sv * cv;
        [
            [[0.0, gamma_0_01], [gamma_0_01, 0.0]],
            [[gamma_1_00, 0.0], [0.0, 0.0]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        let u = u.rem_euclid(TAU);
        let v = v.clamp(0.01, PI - 0.01);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        self.position(u, v).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(0.0..TAU), rng.gen_range(0.1..PI - 0.1))
    }

    fn random_tangent(&self, _u: f32, _v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let speed = 0.5f32;
        (angle.cos() * speed, angle.sin() * speed)
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=u_steps {
            for j in 0..=v_steps {
                let u = (i as f32 / u_steps as f32) * TAU;
                let v = 0.01 + (j as f32 / v_steps as f32) * (PI - 0.02);
                let p = self.position(u, v);
                verts.push([p.x, p.y, p.z]);
            }
        }
        for i in 0..u_steps {
            for j in 0..v_steps {
                let a = i * (v_steps + 1) + j;
                let b = a + 1;
                let c = (i + 1) * (v_steps + 1) + j;
                let d = c + 1;
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        (verts, indices)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_on_unit_sphere_has_unit_length() {
        let s = Sphere::new(1.0);
        for (u, v) in [(0.0, PI / 2.0), (1.0, 1.0), (3.0, 2.0)] {
            let p = s.position(u, v);
            assert!((p.length() - 1.0).abs() < 1e-5, "r={}", p.length());
        }
    }

    #[test]
    fn position_scales_with_radius() {
        let s2 = Sphere::new(2.5);
        let p = s2.position(0.0, PI / 2.0);
        assert!((p.length() - 2.5).abs() < 1e-5);
    }

    #[test]
    fn metric_is_symmetric() {
        let s = Sphere::new(1.0);
        let g = s.metric(0.5, 1.0);
        assert!((g[0][1] - g[1][0]).abs() < 1e-6);
    }

    #[test]
    fn christoffel_at_pole_is_zero() {
        // Near the poles v ≈ 0 or v ≈ π, sin(v) ≈ 0 so Γ^0_01 is clamped to 0.
        let s = Sphere::new(1.0);
        let g = s.christoffel(0.0, 1e-8);
        assert!(g[0][0][1].abs() < 1.0, "Γ^0_01 should not blow up near pole");
    }

    #[test]
    fn christoffel_at_equator_gamma_1_00() {
        // At v=PI/2: Γ^1_00 = -sin(v)cos(v) = -1*0 = 0.
        let s = Sphere::new(1.0);
        let g = s.christoffel(0.0, PI / 2.0);
        assert!(g[1][0][0].abs() < 1e-5, "Γ^1_00 at equator: {}", g[1][0][0]);
    }

    #[test]
    fn wrap_clamps_v_and_wraps_u() {
        let s = Sphere::new(1.0);
        let (u, v) = s.wrap(TAU + 0.5, PI + 1.0);
        assert!((0.0..TAU).contains(&u), "u={u}");
        assert!(v <= PI - 0.01, "v not clamped: {v}");
    }

    #[test]
    fn normal_is_radial_on_unit_sphere() {
        let s = Sphere::new(1.0);
        let p = s.position(0.5, 1.0);
        let n = s.normal(0.5, 1.0);
        // Normal of the unit sphere should equal the position vector.
        assert!((n.x - p.x).abs() < 1e-5);
        assert!((n.y - p.y).abs() < 1e-5);
        assert!((n.z - p.z).abs() < 1e-5);
    }

    #[test]
    fn mesh_vertex_count() {
        let s = Sphere::new(1.0);
        let (verts, indices) = s.mesh_vertices(8, 8);
        assert_eq!(verts.len(), 9 * 9);
        assert_eq!(indices.len(), 8 * 8 * 6);
    }

    /// At the equator `v = π/2`, the metric should be `g_00 = r² sin²(π/2) = r²`
    /// and `g_11 = r²`, with `g_01 = 0`.
    #[test]
    fn test_sphere_metric_at_equator() {
        let r = 2.5f32;
        let s = Sphere::new(r);
        let g = s.metric(0.0, PI / 2.0);
        let expected = r * r;
        assert!((g[0][0] - expected).abs() < 1e-4,
            "g_00 = {} but expected r² = {expected}", g[0][0]);
        assert!((g[1][1] - expected).abs() < 1e-4,
            "g_11 = {} but expected r² = {expected}", g[1][1]);
        assert!(g[0][1].abs() < 1e-5,
            "g_01 = {} should be 0 at equator", g[0][1]);
    }

    /// A geodesic launched along the equator (`v = π/2`, `dv = 0`) should stay
    /// near `v = π/2` because the equator is a geodesic by symmetry.
    #[test]
    fn test_sphere_geodesic_great_circle() {
        use crate::geodesic::Geodesic;
        let s = Sphere::new(1.0);
        // At v=PI/2: Γ^1_00 = -sin(v)cos(v) = 0, so dv stays 0 → v stays π/2.
        let mut geo = Geodesic::new(0.0, PI / 2.0, 0.5, 0.0, 1000, 0);

        for _ in 0..200 {
            geo.step(&s, 0.016);
        }

        // v should remain very close to π/2 (equator).
        assert!((geo.v - PI / 2.0).abs() < 0.05,
            "v drifted from equator: v = {}", geo.v);
    }

    /// The outward unit normal must have length 1 at every sampled point.
    ///
    /// For the sphere the normal equals the position vector (normalised), so
    /// this also checks that `position` is consistent with `normal`.
    #[test]
    fn test_sphere_normal_unit_length() {
        let s = Sphere::new(2.5);
        for ui in 0..8u32 {
            for vi in 1..8u32 { // skip poles where v=0 and v=PI
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * PI / 8.0;
                let n = s.normal(u, v);
                assert!((n.length() - 1.0).abs() < 1e-5,
                    "normal not unit at u={u:.3} v={v:.3}: |n|={}", n.length());
            }
        }
    }

    /// The metric must be positive definite (both diagonal entries > 0 and
    /// det > 0) at every sampled point away from the poles.
    #[test]
    fn test_sphere_metric_positive_definite() {
        let s = Sphere::new(1.0);
        for ui in 0..8u32 {
            for vi in 1..8u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * PI / 8.0;
                let g = s.metric(u, v);
                assert!(g[0][0] > 0.0, "g_00 <= 0 at u={u:.3} v={v:.3}");
                assert!(g[1][1] > 0.0, "g_11 <= 0 at u={u:.3} v={v:.3}");
                let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
                assert!(det > 0.0, "det(g) <= 0 at u={u:.3} v={v:.3}: det={det}");
            }
        }
    }

    /// Christoffel symbols must satisfy the torsion-free symmetry Γ^k_ij = Γ^k_ji
    /// at every sampled point.
    #[test]
    fn test_sphere_christoffel_symmetry() {
        let s = Sphere::new(1.0);
        for ui in 0..6u32 {
            for vi in 1..6u32 {
                let u = ui as f32 * TAU / 6.0;
                let v = vi as f32 * PI / 6.0;
                let g = s.christoffel(u, v);
                for k in 0..2 {
                    assert!((g[k][0][1] - g[k][1][0]).abs() < 1e-6,
                        "Γ^{k}_01 != Γ^{k}_10 at u={u:.3} v={v:.3}");
                }
            }
        }
    }
}
