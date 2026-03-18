//! Hyperbolic paraboloid (saddle) surface with analytic Christoffel symbols.

use super::Surface;
use glam::Vec3;

/// Hyperbolic paraboloid (saddle surface) with the embedding `z = (u^2 - v^2) / scale`.
///
/// Parameter domain: `u, v in [-2, 2]`. This surface has everywhere negative
/// Gaussian curvature, causing geodesics to diverge exponentially.
///
/// The Christoffel symbols are computed analytically from the metric tensor,
/// which is not diagonal due to the off-diagonal coupling terms from `g_01`.
pub struct Saddle {
    /// Denominator in the saddle equation `z = (u^2 - v^2) / scale`.
    ///
    /// Larger values flatten the saddle; smaller values make it steeper.
    pub scale: f32,
}

impl Saddle {
    /// Construct a saddle surface with the given `scale` parameter.
    ///
    /// A `scale` of `2.0` produces a moderate saddle suitable for wallpaper use.
    pub fn new(scale: f32) -> Self { Self { scale } }

    /// Compute the 3D embedding `(u, v, (u²-v²)/scale)`.
    fn embed(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(u, v, (u * u - v * v) / self.scale)
    }

    /// First partial derivative `∂φ/∂u` — depends only on `u`.
    fn d_du(&self, u: f32, _v: f32) -> Vec3 {
        Vec3::new(1.0, 0.0, 2.0 * u / self.scale)
    }

    /// First partial derivative `∂φ/∂v` — depends only on `v`.
    fn d_dv(&self, _u: f32, v: f32) -> Vec3 {
        Vec3::new(0.0, 1.0, -2.0 * v / self.scale)
    }
}

impl Surface for Saddle {
    fn position(&self, u: f32, v: f32) -> Vec3 { self.embed(u, v) }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Metric: g_00 = 1 + 4u²/a², g_01 = -4uv/a², g_11 = 1 + 4v²/a²
        let a2 = self.scale * self.scale;
        let g00 = 1.0 + 4.0 * u * u / a2;
        let g01 = -4.0 * u * v / a2;
        let g11 = 1.0 + 4.0 * v * v / a2;
        let det = g00 * g11 - g01 * g01;
        let inv00 = g11 / det;
        let inv01 = -g01 / det;
        let inv11 = g00 / det;

        // Derivatives of metric:
        // ∂_u g_00 = 8u/a², ∂_v g_00 = 0
        // ∂_u g_01 = -4v/a², ∂_v g_01 = -4u/a²
        // ∂_u g_11 = 0, ∂_v g_11 = 8v/a²
        let dg00_du = 8.0 * u / a2;
        let dg01_du = -4.0 * v / a2;
        let dg01_dv = -4.0 * u / a2;
        let dg11_dv = 8.0 * v / a2;

        // Γ^k_ij = (1/2) g^{kl} (∂_i g_{lj} + ∂_j g_{li} - ∂_l g_{ij})
        let half_dg = |i: usize, j: usize, l: usize| -> f32 {
            // Returns ∂_i g_{lj} + ∂_j g_{li} - ∂_l g_{ij}
            let dg_da = |a: usize, b: usize, coord: usize| -> f32 {
                match (a.min(b), a.max(b), coord) {
                    (0, 0, 0) => dg00_du,
                    (0, 1, 0) => dg01_du,
                    (0, 1, 1) => dg01_dv,
                    (1, 1, 1) => dg11_dv,
                    _ => 0.0,
                }
            };
            dg_da(l, j, i) + dg_da(l, i, j) - dg_da(i, j, l)
        };

        let mut gamma = [[[0.0f32; 2]; 2]; 2];
        for k in 0..2usize {
            for i in 0..2usize {
                for j in 0..2usize {
                    let sum: f32 = (0..2).map(|l| {
                        let ginv = match (k, l) {
                            (0, 0) => inv00,
                            (0, 1) | (1, 0) => inv01,
                            (1, 1) => inv11,
                            _ => 0.0,
                        };
                        ginv * half_dg(i, j, l)
                    }).sum();
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
        let s = Saddle::new(2.0);
        let p = s.position(0.0, 0.0);
        assert!(p.x.abs() < 1e-6);
        assert!(p.y.abs() < 1e-6);
        assert!(p.z.abs() < 1e-6);
    }

    #[test]
    fn position_formula_correct() {
        // z = (u² - v²) / scale
        let s = Saddle::new(2.0);
        let p = s.position(1.0, 1.0);
        assert!((p.x - 1.0).abs() < 1e-6);
        assert!((p.y - 1.0).abs() < 1e-6);
        assert!(p.z.abs() < 1e-6, "z = {} (expected 0 for u=v=1)", p.z);

        let p2 = s.position(2.0, 0.0);
        assert!((p2.z - 4.0 / 2.0).abs() < 1e-6, "z={}", p2.z);
    }

    #[test]
    fn metric_at_origin_is_identity() {
        // At u=0,v=0: g_00=1, g_01=0, g_11=1.
        let s = Saddle::new(2.0);
        let g = s.metric(0.0, 0.0);
        assert!((g[0][0] - 1.0).abs() < 1e-6);
        assert!(g[0][1].abs() < 1e-6);
        assert!((g[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metric_is_symmetric() {
        let s = Saddle::new(2.0);
        let g = s.metric(0.5, 1.2);
        assert!((g[0][1] - g[1][0]).abs() < 1e-6);
    }

    #[test]
    fn wrap_clamps_to_bounds() {
        let s = Saddle::new(2.0);
        let (u, v) = s.wrap(5.0, -5.0);
        assert!((u - 2.0).abs() < 1e-6, "u={u}");
        assert!((v + 2.0).abs() < 1e-6, "v={v}");
    }

    #[test]
    fn christoffel_symmetry() {
        let s = Saddle::new(2.0);
        let g = s.christoffel(0.4, 0.8);
        for k in 0..2 {
            assert!((g[k][0][1] - g[k][1][0]).abs() < 1e-5,
                "Γ^{k}_01 != Γ^{k}_10");
        }
    }

    #[test]
    fn christoffel_at_origin_is_zero() {
        // At u=0,v=0: all metric derivatives vanish → Christoffels all zero.
        let s = Saddle::new(2.0);
        let g = s.christoffel(0.0, 0.0);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(g[k][i][j].abs() < 1e-5,
                        "Γ^{k}_{i}{j}={} at origin", g[k][i][j]);
                }
            }
        }
    }

    #[test]
    fn normal_is_unit() {
        let s = Saddle::new(2.0);
        let n = s.normal(0.5, 0.5);
        assert!((n.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn mesh_vertex_count() {
        let s = Saddle::new(2.0);
        let (verts, indices) = s.mesh_vertices(6, 6);
        assert_eq!(verts.len(), 7 * 7);
        assert_eq!(indices.len(), 6 * 6 * 6);
    }

    #[test]
    fn near_zero_scale_does_not_panic() {
        let s = Saddle::new(0.0001);
        let p = s.position(0.1, 0.1);
        // Position will be finite even with a tiny scale.
        assert!(p.x.is_finite());
    }

    /// The outward unit normal must have length 1 at a grid of sample points.
    #[test]
    fn test_saddle_normal_unit_length() {
        let s = Saddle::new(2.0);
        for ui in 0..5 {
            for vi in 0..5 {
                let u = -1.6 + ui as f32 * 0.8;
                let v = -1.6 + vi as f32 * 0.8;
                let n = s.normal(u, v);
                assert!((n.length() - 1.0).abs() < 1e-5,
                    "normal not unit at u={u:.2} v={v:.2}: |n|={}", n.length());
            }
        }
    }

    /// The metric must be positive definite at every sampled point.
    #[test]
    fn test_saddle_metric_positive_definite() {
        let s = Saddle::new(2.0);
        for ui in 0..5 {
            for vi in 0..5 {
                let u = -1.6 + ui as f32 * 0.8;
                let v = -1.6 + vi as f32 * 0.8;
                let g = s.metric(u, v);
                assert!(g[0][0] > 0.0, "g_00 <= 0 at u={u:.2} v={v:.2}");
                assert!(g[1][1] > 0.0, "g_11 <= 0 at u={u:.2} v={v:.2}");
                let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
                assert!(det > 0.0, "det(g) <= 0 at u={u:.2} v={v:.2}: det={det}");
            }
        }
    }

    /// At a non-origin point the Christoffel symbols should be non-zero and
    /// still satisfy torsion-free symmetry Γ^k_ij = Γ^k_ji.
    #[test]
    fn test_saddle_christoffel_nonzero_and_symmetric_off_origin() {
        let s = Saddle::new(2.0);
        let g = s.christoffel(1.0, 0.5);
        // At least one symbol should be non-zero away from origin.
        let any_nonzero = (0..2).any(|k| (0..2).any(|i| (0..2).any(|j| g[k][i][j].abs() > 1e-6)));
        assert!(any_nonzero, "all Christoffel symbols zero at non-origin point");
        // Still must be symmetric.
        for k in 0..2 {
            assert!((g[k][0][1] - g[k][1][0]).abs() < 1e-5,
                "Γ^{k}_01 != Γ^{k}_10 at (1.0, 0.5)");
        }
    }
}
