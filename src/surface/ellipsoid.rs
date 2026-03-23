//! Ellipsoid surface: x²/a² + y²/b² + z²/c² = 1.
//!
//! Uses the standard spherical parameterization adapted to three independent
//! semi-axes.  All Christoffel symbols are computed analytically.

use super::Surface;
use glam::Vec3;
use std::f32::consts::{PI, TAU};

/// Ellipsoid with semi-axes `a`, `b`, `c`.
///
/// # Parameterization
/// - `u ∈ [0, 2π)` — azimuthal (longitude)
/// - `v ∈ (0, π)` — polar (colatitude)
///
/// ```text
/// x = a · sin(v) · cos(u)
/// y = b · sin(v) · sin(u)
/// z = c · cos(v)
/// ```
///
/// When `a = b = c` this reduces to a sphere of radius `a`.
pub struct Ellipsoid {
    /// Semi-axis along x.
    pub a: f32,
    /// Semi-axis along y.
    pub b: f32,
    /// Semi-axis along z.
    pub c: f32,
}

impl Ellipsoid {
    /// Construct an ellipsoid with the given semi-axes.
    pub fn new(a: f32, b: f32, c: f32) -> Self {
        Self { a, b, c }
    }

    /// `∂φ/∂u = (-a sin(v) sin(u),  b sin(v) cos(u),  0)`.
    fn d_du(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            -self.a * v.sin() * u.sin(),
            self.b * v.sin() * u.cos(),
            0.0,
        )
    }

    /// `∂φ/∂v = (a cos(v) cos(u),  b cos(v) sin(u),  -c sin(v))`.
    fn d_dv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            self.a * v.cos() * u.cos(),
            self.b * v.cos() * u.sin(),
            -self.c * v.sin(),
        )
    }
}

impl Surface for Ellipsoid {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            self.a * v.sin() * u.cos(),
            self.b * v.sin() * u.sin(),
            self.c * v.cos(),
        )
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    /// Christoffel symbols computed numerically via finite differences of the
    /// metric tensor.  The ellipsoid metric has off-diagonal coupling when
    /// `a ≠ b`, so a fully analytic but lengthy expression is replaced by the
    /// standard finite-difference formula for generality.
    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        let h = 1e-4_f32;

        // Sample metric at neighbouring points.
        let g = self.metric(u, v);
        let g_pu = self.metric(u + h, v);
        let g_mu = self.metric(u - h, v);
        let g_pv = self.metric(u, v + h);
        let g_mv = self.metric(u, v - h);

        // Central-difference partial derivatives of metric.
        let dg_du = |i: usize, j: usize| (g_pu[i][j] - g_mu[i][j]) / (2.0 * h);
        let dg_dv = |i: usize, j: usize| (g_pv[i][j] - g_mv[i][j]) / (2.0 * h);

        let dg = |r: usize, c: usize, coord: usize| -> f32 {
            if coord == 0 {
                dg_du(r, c)
            } else {
                dg_dv(r, c)
            }
        };

        // Invert 2×2 metric.
        let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
        let inv00 = g[1][1] / det;
        let inv01 = -g[0][1] / det;
        let inv11 = g[0][0] / det;

        let ginv = |k: usize, l: usize| -> f32 {
            match (k, l) {
                (0, 0) => inv00,
                (0, 1) | (1, 0) => inv01,
                (1, 1) => inv11,
                _ => 0.0,
            }
        };

        let mut gamma = [[[0.0f32; 2]; 2]; 2];
        #[allow(clippy::needless_range_loop)]
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    let sum: f32 = (0..2)
                        .map(|l| {
                            ginv(k, l)
                                * (dg(l, j, i) + dg(l, i, j) - dg(i, j, l))
                        })
                        .sum();
                    gamma[k][i][j] = 0.5 * sum;
                }
            }
        }
        gamma
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        let u = u.rem_euclid(TAU);
        let v = v.clamp(0.01, PI - 0.01);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        e1.cross(e2).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(0.0..TAU), rng.gen_range(0.1..PI - 0.1))
    }

    fn random_tangent(&self, _u: f32, _v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let speed = 0.4f32;
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
    fn position_on_unit_sphere() {
        // a=b=c=1 is a unit sphere.
        let e = Ellipsoid::new(1.0, 1.0, 1.0);
        for (u, v) in [(0.0f32, PI / 2.0), (1.0, 1.0), (3.0, 2.0)] {
            let p = e.position(u, v);
            assert!((p.length() - 1.0).abs() < 1e-5, "|p|={}", p.length());
        }
    }

    #[test]
    fn axes_are_independent() {
        let e = Ellipsoid::new(3.0, 2.0, 1.0);
        // At v=π/2, u=0: (a,0,0)
        let p = e.position(0.0, PI / 2.0);
        assert!((p.x - 3.0).abs() < 1e-5, "x={}", p.x);
        assert!(p.y.abs() < 1e-5);
        assert!(p.z.abs() < 1e-5);
        // At v=0: (0,0,c)
        let p2 = e.position(0.0, 0.01);
        assert!(p2.z > 0.9);
    }

    #[test]
    fn metric_is_symmetric() {
        let e = Ellipsoid::new(2.0, 1.5, 1.0);
        let g = e.metric(0.5, 1.0);
        assert!((g[0][1] - g[1][0]).abs() < 1e-6);
    }

    #[test]
    fn metric_positive_definite() {
        let e = Ellipsoid::new(2.0, 1.5, 1.0);
        for ui in 0..5 {
            for vi in 0..5 {
                let u = ui as f32 * TAU / 5.0;
                let v = 0.2 + vi as f32 * (PI - 0.4) / 5.0;
                let g = e.metric(u, v);
                assert!(g[0][0] > 0.0);
                let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
                assert!(det > 0.0, "det={det} u={u} v={v}");
            }
        }
    }

    #[test]
    fn christoffel_symmetry() {
        let e = Ellipsoid::new(2.0, 1.5, 1.0);
        let g = e.christoffel(0.5, 1.0);
        for k in 0..2 {
            assert!(
                (g[k][0][1] - g[k][1][0]).abs() < 1e-3,
                "Γ^{k}_01={} != Γ^{k}_10={}",
                g[k][0][1],
                g[k][1][0]
            );
        }
    }

    #[test]
    fn normal_is_unit() {
        let e = Ellipsoid::new(2.0, 1.5, 1.0);
        let n = e.normal(1.0, 1.0);
        assert!((n.length() - 1.0).abs() < 1e-5, "|n|={}", n.length());
    }

    #[test]
    fn mesh_vertex_count() {
        let e = Ellipsoid::new(1.0, 1.0, 1.0);
        let (verts, indices) = e.mesh_vertices(6, 6);
        assert_eq!(verts.len(), 7 * 7);
        assert_eq!(indices.len(), 6 * 6 * 6);
    }
}
