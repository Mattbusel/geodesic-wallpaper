//! Hyperbolic paraboloid (saddle+) surface: z = x²/a² - y²/b².
//!
//! Unlike the simpler `Saddle` surface which uses `z = (u²-v²)/scale`, this
//! implementation exposes independent `a` and `b` parameters so the curvature
//! in each direction can be tuned separately.  All Christoffel symbols are
//! computed analytically from the metric tensor.

use super::Surface;
use glam::Vec3;

/// Hyperbolic paraboloid with embedding `z = u²/a² - v²/b²`.
///
/// # Parameterization
/// - `u ∈ [-2, 2]`
/// - `v ∈ [-2, 2]`
///
/// Setting `a = b` recovers a symmetric saddle.  Large `a` or `b` flatten the
/// curvature in the corresponding direction; small values sharpen it.
pub struct HyperbolicParaboloid {
    /// Controls curvature along the `u` (x) axis: `z = u²/a² - …`.
    pub a: f32,
    /// Controls curvature along the `v` (y) axis: `z = … - v²/b²`.
    pub b: f32,
}

impl HyperbolicParaboloid {
    /// Construct a hyperbolic paraboloid with the given scale parameters.
    ///
    /// Both `a` and `b` must be non-zero; panicking behaviour on zero is the
    /// caller's responsibility.
    pub fn new(a: f32, b: f32) -> Self {
        Self { a, b }
    }

    /// `∂φ/∂u = (1, 0, 2u/a²)`.
    fn d_du(&self, u: f32, _v: f32) -> Vec3 {
        Vec3::new(1.0, 0.0, 2.0 * u / (self.a * self.a))
    }

    /// `∂φ/∂v = (0, 1, -2v/b²)`.
    fn d_dv(&self, _u: f32, v: f32) -> Vec3 {
        Vec3::new(0.0, 1.0, -2.0 * v / (self.b * self.b))
    }
}

impl Surface for HyperbolicParaboloid {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(u, v, u * u / (self.a * self.a) - v * v / (self.b * self.b))
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        let a2 = self.a * self.a;
        let b2 = self.b * self.b;

        // Metric components:
        // g_00 = 1 + 4u²/a⁴,  g_01 = -4uv/(a²b²),  g_11 = 1 + 4v²/b⁴
        let g00 = 1.0 + 4.0 * u * u / (a2 * a2);
        let g01 = -4.0 * u * v / (a2 * b2);
        let g11 = 1.0 + 4.0 * v * v / (b2 * b2);
        let det = g00 * g11 - g01 * g01;
        let inv00 = g11 / det;
        let inv01 = -g01 / det;
        let inv11 = g00 / det;

        // Partial derivatives of metric components:
        // ∂_u g_00 = 8u/a⁴,  ∂_v g_00 = 0
        // ∂_u g_01 = -4v/(a²b²),  ∂_v g_01 = -4u/(a²b²)
        // ∂_u g_11 = 0,  ∂_v g_11 = 8v/b⁴
        let dg00_du = 8.0 * u / (a2 * a2);
        let dg01_du = -4.0 * v / (a2 * b2);
        let dg01_dv = -4.0 * u / (a2 * b2);
        let dg11_dv = 8.0 * v / (b2 * b2);

        let dg = |row: usize, col: usize, coord: usize| -> f32 {
            let (r, c) = (row.min(col), row.max(col));
            match (r, c, coord) {
                (0, 0, 0) => dg00_du,
                (0, 1, 0) => dg01_du,
                (0, 1, 1) => dg01_dv,
                (1, 1, 1) => dg11_dv,
                _ => 0.0,
            }
        };

        let mut gamma = [[[0.0f32; 2]; 2]; 2];
        #[allow(clippy::needless_range_loop)]
        for k in 0..2usize {
            for i in 0..2usize {
                for j in 0..2usize {
                    let sum: f32 = (0..2)
                        .map(|l| {
                            let ginv = match (k, l) {
                                (0, 0) => inv00,
                                (0, 1) | (1, 0) => inv01,
                                (1, 1) => inv11,
                                _ => 0.0,
                            };
                            // Γ^k_ij = ½ g^{kl}(∂_i g_{lj} + ∂_j g_{li} − ∂_l g_{ij})
                            ginv * (dg(l, j, i) + dg(l, i, j) - dg(i, j, l))
                        })
                        .sum();
                    gamma[k][i][j] = 0.5 * sum;
                }
            }
        }
        gamma
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        (u.clamp(-2.0, 2.0), v.clamp(-2.0, 2.0))
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        e1.cross(e2).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(-1.8f32..1.8), rng.gen_range(-1.8f32..1.8))
    }

    fn random_tangent(&self, _u: f32, _v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = 0.3f32;
        (angle.cos() * speed, angle.sin() * speed)
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=u_steps {
            for j in 0..=v_steps {
                let u = -2.0 + (i as f32 / u_steps as f32) * 4.0;
                let v = -2.0 + (j as f32 / v_steps as f32) * 4.0;
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
    fn position_at_origin_is_zero() {
        let s = HyperbolicParaboloid::new(1.0, 1.0);
        let p = s.position(0.0, 0.0);
        assert!(p.x.abs() < 1e-6);
        assert!(p.y.abs() < 1e-6);
        assert!(p.z.abs() < 1e-6);
    }

    #[test]
    fn asymmetric_axes_produce_correct_z() {
        // z = u²/a² - v²/b²  with a=2, b=1: at (2,1) → 4/4 - 1/1 = 0
        let s = HyperbolicParaboloid::new(2.0, 1.0);
        let p = s.position(2.0, 1.0);
        assert!(p.z.abs() < 1e-5, "z={}", p.z);
        // at (2,0) → 4/4 = 1
        let p2 = s.position(2.0, 0.0);
        assert!((p2.z - 1.0).abs() < 1e-5, "z={}", p2.z);
    }

    #[test]
    fn metric_is_symmetric() {
        let s = HyperbolicParaboloid::new(1.5, 2.0);
        let g = s.metric(0.7, -0.5);
        assert!((g[0][1] - g[1][0]).abs() < 1e-6);
    }

    #[test]
    fn metric_is_positive_definite() {
        let s = HyperbolicParaboloid::new(1.0, 1.0);
        for ui in 0..5 {
            for vi in 0..5 {
                let u = -1.6 + ui as f32 * 0.8;
                let v = -1.6 + vi as f32 * 0.8;
                let g = s.metric(u, v);
                assert!(g[0][0] > 0.0);
                let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
                assert!(det > 0.0, "det={det} at u={u} v={v}");
            }
        }
    }

    #[test]
    fn christoffel_symmetry() {
        let s = HyperbolicParaboloid::new(1.0, 1.0);
        let g = s.christoffel(0.5, 0.8);
        for k in 0..2 {
            assert!(
                (g[k][0][1] - g[k][1][0]).abs() < 1e-5,
                "Γ^{k}_01 != Γ^{k}_10"
            );
        }
    }

    #[test]
    fn normal_is_unit() {
        let s = HyperbolicParaboloid::new(1.0, 1.5);
        let n = s.normal(0.5, -0.5);
        assert!((n.length() - 1.0).abs() < 1e-5, "|n|={}", n.length());
    }

    #[test]
    fn mesh_vertex_count() {
        let s = HyperbolicParaboloid::new(1.0, 1.0);
        let (verts, indices) = s.mesh_vertices(6, 6);
        assert_eq!(verts.len(), 7 * 7);
        assert_eq!(indices.len(), 6 * 6 * 6);
    }
}
