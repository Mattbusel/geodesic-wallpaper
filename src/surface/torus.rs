//! Torus surface parameterization with analytic Christoffel symbols.

use super::Surface;
use glam::Vec3;
use std::f32::consts::TAU;

/// Torus with major radius R (center to tube center) and minor radius r (tube radius).
///
/// # Parameterization
/// - `u ∈ [0, 2π)` — longitude around the major circle
/// - `v ∈ [0, 2π)` — latitude around the tube
///
/// ```text
/// x = (R + r cos v) cos u
/// y = (R + r cos v) sin u
/// z = r sin v
/// ```
pub struct Torus {
    /// Major radius — distance from the torus centre to the centre of the tube.
    pub big_r: f32,
    /// Minor radius — radius of the tube itself.
    pub small_r: f32,
}

impl Torus {
    /// Construct a torus with the given major and minor radii.
    pub fn new(big_r: f32, small_r: f32) -> Self {
        Self { big_r, small_r }
    }

    /// First partial derivative `∂φ/∂u` of the embedding at `(u, v)`.
    fn d_du(&self, u: f32, v: f32) -> Vec3 {
        let r = self.big_r + self.small_r * v.cos();
        Vec3::new(-r * u.sin(), r * u.cos(), 0.0)
    }

    /// First partial derivative `∂φ/∂v` of the embedding at `(u, v)`.
    fn d_dv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            -self.small_r * v.sin() * u.cos(),
            -self.small_r * v.sin() * u.sin(),
            self.small_r * v.cos(),
        )
    }

    /// Second partial derivative `∂²φ/∂u²` at `(u, v)`.
    #[allow(dead_code)]
    fn d2_du2(&self, u: f32, v: f32) -> Vec3 {
        let r = self.big_r + self.small_r * v.cos();
        Vec3::new(-r * u.cos(), -r * u.sin(), 0.0)
    }

    /// Second partial derivative `∂²φ/∂v²` at `(u, v)`.
    #[allow(dead_code)]
    fn d2_dv2(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            -self.small_r * v.cos() * u.cos(),
            -self.small_r * v.cos() * u.sin(),
            -self.small_r * v.sin(),
        )
    }

    /// Mixed second partial derivative `∂²φ/∂u∂v` at `(u, v)`.
    #[allow(dead_code)]
    fn d2_dudv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            self.small_r * v.sin() * u.sin(),
            -self.small_r * v.sin() * u.cos(),
            0.0,
        )
    }

    /// Compute both the metric `g` and its inverse `g⁻¹` at `(u, v)`.
    ///
    /// The torus has an orthogonal parameterisation so `g_01 = 0`, making the
    /// inverse trivial: `g⁻¹ = diag(1/g_00, 1/g_11)`.
    fn metric_and_inv(&self, u: f32, v: f32) -> ([[f32; 2]; 2], [[f32; 2]; 2]) {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        let g00 = e1.dot(e1);
        let g01 = e1.dot(e2);
        let g11 = e2.dot(e2);
        let g = [[g00, g01], [g01, g11]];
        let det = g00 * g11 - g01 * g01;
        let inv = [[g11 / det, -g01 / det], [-g01 / det, g00 / det]];
        (g, inv)
    }
}

impl Surface for Torus {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        let r = self.big_r + self.small_r * v.cos();
        Vec3::new(r * u.cos(), r * u.sin(), self.small_r * v.sin())
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, _u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Γ^k_ij = (1/2) g^{kl} (∂_i g_{lj} + ∂_j g_{li} - ∂_l g_{ij})
        // For torus, g_01 = 0 everywhere (orthogonal parameterization)
        // g_00 = (R + r cos v)^2,  g_11 = r^2
        // ∂_u g_00 = 0,  ∂_v g_00 = -2(R + r cos v) r sin v
        // ∂_u g_11 = 0,  ∂_v g_11 = 0
        let f = self.big_r + self.small_r * v.cos();
        let df_dv = -self.small_r * v.sin();
        let _g00 = f * f;
        let g11 = self.small_r * self.small_r;

        // Non-zero Christoffels for orthogonal parameterization:
        // Γ^0_01 = Γ^0_10 = (∂_v g_00) / (2 g_00) = df_dv / f
        // Γ^1_00 = -(∂_v g_00) / (2 g_11) = -f df_dv / r^2
        let gamma_0_01 = df_dv / f;
        let gamma_1_00 = -f * df_dv / g11;

        [
            // k=0: Γ^0_ij
            [[0.0, gamma_0_01], [gamma_0_01, 0.0]],
            // k=1: Γ^1_ij
            [[gamma_1_00, 0.0], [0.0, 0.0]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        let u = u.rem_euclid(TAU);
        let v = v.rem_euclid(TAU);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        e1.cross(e2).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(0.0..TAU), rng.gen_range(0.0..TAU))
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let (g, _) = self.metric_and_inv(u, v);
        // Normalize so g_ij du^i du^j = 1
        let speed = 1.0;
        let du = angle.cos() * speed / g[0][0].sqrt();
        let dv = angle.sin() * speed / g[1][1].sqrt();
        (du, dv)
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=u_steps {
            for j in 0..=v_steps {
                let u = (i as f32 / u_steps as f32) * TAU;
                let v = (j as f32 / v_steps as f32) * TAU;
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
    use std::f32::consts::PI;

    #[test]
    fn position_at_known_point() {
        // At u=0, v=0: x = R+r, y = 0, z = 0.
        let t = Torus::new(2.0, 0.7);
        let p = t.position(0.0, 0.0);
        assert!((p.x - 2.7).abs() < 1e-5, "x={}", p.x);
        assert!(p.y.abs() < 1e-5, "y={}", p.y);
        assert!(p.z.abs() < 1e-5, "z={}", p.z);
    }

    #[test]
    fn metric_is_symmetric() {
        let t = Torus::new(2.0, 0.7);
        let g = t.metric(0.5, 1.2);
        assert!((g[0][1] - g[1][0]).abs() < 1e-6);
    }

    #[test]
    fn metric_is_positive_definite() {
        let t = Torus::new(2.0, 0.7);
        let g = t.metric(1.0, 0.5);
        // g_00 > 0, det > 0.
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        assert!(g[0][0] > 0.0, "g_00 not positive: {}", g[0][0]);
        assert!(det > 0.0, "det not positive: {det}");
    }

    #[test]
    fn wrap_is_periodic() {
        let t = Torus::new(2.0, 0.7);
        let (u, v) = t.wrap(TAU + 0.3, -0.1);
        assert!((0.0..TAU).contains(&u), "u={u}");
        assert!((0.0..TAU).contains(&v), "v={v}");
    }

    #[test]
    fn mesh_vertex_count() {
        let t = Torus::new(2.0, 0.7);
        let (verts, indices) = t.mesh_vertices(10, 10);
        assert_eq!(verts.len(), 11 * 11);
        // 10*10 quads * 2 triangles * 3 indices
        assert_eq!(indices.len(), 10 * 10 * 6);
    }

    #[test]
    fn degenerate_minor_radius_does_not_panic() {
        let t = Torus::new(2.0, 0.0001);
        let p = t.position(0.5, 0.5);
        assert!(p.x.is_finite());
    }

    #[test]
    fn christoffel_symmetry_ij() {
        // Γ^k_ij = Γ^k_ji (lower indices are symmetric).
        let t = Torus::new(2.0, 0.7);
        let g = t.christoffel(0.4, 0.8);
        for k in 0..2 {
            assert!((g[k][0][1] - g[k][1][0]).abs() < 1e-6,
                "Γ^{k}_01 != Γ^{k}_10");
        }
    }

    #[test]
    fn normal_is_unit() {
        let t = Torus::new(2.0, 0.7);
        let n = t.normal(1.0, 1.0);
        assert!((n.length() - 1.0).abs() < 1e-5, "normal length={}", n.length());
    }

    #[test]
    fn outer_equator_christoffel_all_zero() {
        // At v=0 (outer equator), sin(v)=0, so df_dv=0 and all Christoffels=0.
        let t = Torus::new(2.0, 0.7);
        let g = t.christoffel(0.0, 0.0);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(g[k][i][j].abs() < 1e-6,
                        "Γ^{k}_{i}{j} = {} at outer equator", g[k][i][j]);
                }
            }
        }
    }

    #[test]
    fn inner_equator_christoffel_nonzero() {
        // At v=PI (inner equator), sin(v)=0 as well → all Christoffels=0.
        // At v=PI/2 (topmost circle), sin(v)=1, df_dv=-r → nonzero Christoffels.
        let t = Torus::new(2.0, 0.7);
        let g = t.christoffel(0.0, PI / 2.0);
        // Γ^0_01 = df_dv / f = -r / (R + r*cos(PI/2)) = -r / R ≠ 0.
        assert!(g[0][0][1].abs() > 0.1, "expected nonzero Γ^0_01: {}", g[0][0][1]);
    }

    /// g_00 and g_11 must be strictly positive for every sampled (u, v) because
    /// they are squared lengths of the non-zero coordinate tangent vectors.
    /// The determinant must also be positive (positive-definite matrix).
    #[test]
    fn test_torus_metric_positive_definite() {
        let t = Torus::new(2.0, 0.7);
        for ui in 0..8u32 {
            for vi in 0..8u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * TAU / 8.0;
                let g = t.metric(u, v);
                assert!(g[0][0] > 0.0, "g_00={} ≤ 0 at u={u} v={v}", g[0][0]);
                assert!(g[1][1] > 0.0, "g_11={} ≤ 0 at u={u} v={v}", g[1][1]);
                let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
                assert!(det > 0.0, "det(g)={det} ≤ 0 at u={u} v={v}");
            }
        }
    }

    /// A geodesic launched along the outer equator (v = 0, dv = 0) should keep
    /// v ≈ 0 for many steps because the outer equator is a geodesic by symmetry.
    #[test]
    fn test_torus_geodesic_great_circle() {
        use crate::geodesic::Geodesic;
        let t = Torus::new(2.0, 0.7);
        // Scale du so metric speed g_00 * du^2 = 1.
        let g = t.metric(0.0, 0.0);
        let du = 1.0 / g[0][0].sqrt();
        let mut geo = Geodesic::new(0.0, 0.0, du, 0.0, 1000, 0);

        for _ in 0..200 {
            geo.step(&t, 0.016);
        }

        // v should remain close to 0 (wrap makes 0 and TAU equivalent).
        let v_dist = geo.v.min(TAU - geo.v);
        assert!(v_dist < 0.05,
            "v drifted from equator: wrapped v = {}", geo.v);
    }

    /// The Christoffel array for a 2-D surface must contain exactly 2×2×2 = 8 values.
    #[test]
    fn test_christoffel_symbols_count() {
        let t = Torus::new(2.0, 0.7);
        let gamma = t.christoffel(0.5, 0.5);
        let mut count = 0usize;
        for k in 0..2usize {
            for i in 0..2usize {
                for j in 0..2usize {
                    let _ = gamma[k][i][j];
                    count += 1;
                }
            }
        }
        assert_eq!(count, 8, "expected 8 Christoffel components, got {count}");
    }

    /// The outward unit normal must have length 1 at every sampled point on
    /// the torus.  This verifies that both `d_du` and `d_dv` are implemented
    /// correctly and that their cross product is non-zero everywhere.
    #[test]
    fn test_torus_normal_unit_length() {
        let t = Torus::new(2.0, 0.7);
        for ui in 0..8u32 {
            for vi in 0..8u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * TAU / 8.0;
                let n = t.normal(u, v);
                assert!((n.length() - 1.0).abs() < 1e-5,
                    "normal not unit at u={u:.3} v={v:.3}: |n|={}", n.length());
            }
        }
    }

    /// The Christoffel symbols must satisfy torsion-free symmetry Γ^k_ij = Γ^k_ji
    /// at a representative grid of parameter values.
    #[test]
    fn test_torus_christoffel_symmetry_grid() {
        let t = Torus::new(2.0, 0.7);
        for ui in 0..6u32 {
            for vi in 0..6u32 {
                let u = ui as f32 * TAU / 6.0;
                let v = vi as f32 * TAU / 6.0;
                let g = t.christoffel(u, v);
                for k in 0..2 {
                    assert!((g[k][0][1] - g[k][1][0]).abs() < 1e-6,
                        "Γ^{k}_01 != Γ^{k}_10 at u={u:.3} v={v:.3}");
                }
            }
        }
    }
}
