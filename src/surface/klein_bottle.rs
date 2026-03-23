//! Klein bottle surface — figure-8 immersion in ℝ³.
//!
//! The Klein bottle is a non-orientable closed surface with no boundary.
//! Because it cannot be embedded in ℝ³ without self-intersection we use the
//! **figure-8 immersion** (also called the "pinched torus" form), which is
//! the most commonly visualised version:
//!
//! ```text
//! x = (a + b·cos(v/2)·sin(u) − b·sin(v/2)·sin(2u)) · cos(v)
//! y = (a + b·cos(v/2)·sin(u) − b·sin(v/2)·sin(2u)) · sin(v)
//! z =  b·sin(v/2)·cos(u) + b·cos(v/2)·sin(2u)
//! ```
//!
//! where `u ∈ [0, 2π)` and `v ∈ [0, 2π)`.
//!
//! Christoffel symbols are computed numerically via finite differences of the
//! metric tensor because the analytic form is unwieldy.

use super::Surface;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::TAU;

/// Figure-8 immersion of the Klein bottle in ℝ³.
///
/// # Parameterization
/// - `u ∈ [0, 2π)` — fast winding direction
/// - `v ∈ [0, 2π)` — slow global rotation
///
/// The parameter `a` controls the overall scale; `b` controls the tube radius.
pub struct KleinBottle {
    /// Overall scale parameter.  Default: `2.0`.
    pub a: f32,
    /// Tube radius parameter.  Default: `0.4`.
    pub b: f32,
}

impl KleinBottle {
    /// Construct a Klein bottle with the given shape parameters.
    pub fn new(a: f32, b: f32) -> Self {
        Self {
            a: a.max(0.5),
            b: b.max(0.01),
        }
    }

    fn embed(&self, u: f32, v: f32) -> Vec3 {
        let (su, cu) = u.sin_cos();
        let (sv, cv) = v.sin_cos();
        let (shv, chv) = (v * 0.5).sin_cos();
        let s2u = (2.0 * u).sin();
        let c2u = (2.0 * u).cos();
        let r = self.a + self.b * chv * su - self.b * shv * s2u;
        Vec3::new(r * cv, r * sv, self.b * shv * cu + self.b * chv * s2u)
    }

    fn du(&self, u: f32, v: f32) -> Vec3 {
        const H: f32 = 1e-4;
        let p = self.embed(u + H, v);
        let m = self.embed(u - H, v);
        (p - m) * (0.5 / H)
    }

    fn dv(&self, u: f32, v: f32) -> Vec3 {
        const H: f32 = 1e-4;
        let p = self.embed(u, v + H);
        let m = self.embed(u, v - H);
        (p - m) * (0.5 / H)
    }

    /// Compute metric and its inverse numerically.
    fn metric_and_inv(&self, u: f32, v: f32) -> ([[f32; 2]; 2], [[f32; 2]; 2]) {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        let g00 = eu.dot(eu);
        let g01 = eu.dot(ev);
        let g11 = ev.dot(ev);
        let det = g00 * g11 - g01 * g01;
        let inv_det = if det.abs() > 1e-12 { 1.0 / det } else { 0.0 };
        (
            [[g00, g01], [g01, g11]],
            [[g11 * inv_det, -g01 * inv_det], [-g01 * inv_det, g00 * inv_det]],
        )
    }
}

impl Default for KleinBottle {
    fn default() -> Self {
        Self::new(2.0, 0.4)
    }
}

impl Surface for KleinBottle {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        self.embed(u, v)
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        [[eu.dot(eu), eu.dot(ev)], [eu.dot(ev), ev.dot(ev)]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Numeric second derivatives via finite differences.
        const H: f32 = 1e-3;
        let (g, gi) = self.metric_and_inv(u, v);

        // ∂_u g_ij and ∂_v g_ij via central differences on the metric.
        let metric_uh = self.metric(u + H, v);
        let metric_ul = self.metric(u - H, v);
        let metric_vh = self.metric(u, v + H);
        let metric_vl = self.metric(u, v - H);

        let dg = [
            [
                [(metric_uh[0][0] - metric_ul[0][0]) * 0.5 / H,
                 (metric_vh[0][0] - metric_vl[0][0]) * 0.5 / H],
                [(metric_uh[0][1] - metric_ul[0][1]) * 0.5 / H,
                 (metric_vh[0][1] - metric_vl[0][1]) * 0.5 / H],
            ],
            [
                [(metric_uh[1][0] - metric_ul[1][0]) * 0.5 / H,
                 (metric_vh[1][0] - metric_vl[1][0]) * 0.5 / H],
                [(metric_uh[1][1] - metric_ul[1][1]) * 0.5 / H,
                 (metric_vh[1][1] - metric_vl[1][1]) * 0.5 / H],
            ],
        ];

        let _ = g; // metric values used via gi

        // Γ^k_ij = (1/2) g^{kl} (∂_i g_{lj} + ∂_j g_{li} − ∂_l g_{ij})
        let mut gamma = [[[0.0f32; 2]; 2]; 2];
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    let mut s = 0.0f32;
                    for l in 0..2 {
                        // ∂_i g_{lj}  =  dg[l][j][i]
                        // ∂_j g_{li}  =  dg[l][i][j]
                        // ∂_l g_{ij}  =  dg[i][j][l]   (using symmetry g_ij = g_ji)
                        s += gi[k][l]
                            * (dg[l][j][i] + dg[l][i][j] - dg[i][j][l]);
                    }
                    gamma[k][i][j] = 0.5 * s;
                }
            }
        }
        gamma
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        (u.rem_euclid(TAU), v.rem_euclid(TAU))
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        let n = eu.cross(ev);
        let len = n.length();
        if len > 1e-12 {
            n / len
        } else {
            Vec3::Z
        }
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        (rng.gen_range(0.0..TAU), rng.gen_range(0.0..TAU))
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        let angle: f32 = rng.gen_range(0.0..TAU);
        let du = angle.cos();
        let dv = angle.sin();
        let g = self.metric(u, v);
        let speed_sq = g[0][0] * du * du + 2.0 * g[0][1] * du * dv + g[1][1] * dv * dv;
        if speed_sq > 1e-12 {
            let s = speed_sq.sqrt();
            (du / s, dv / s)
        } else {
            (1.0, 0.0)
        }
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

    #[test]
    fn position_is_finite() {
        let kb = KleinBottle::default();
        for ui in 0..8u32 {
            for vi in 0..8u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * TAU / 8.0;
                let p = kb.position(u, v);
                assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite(),
                    "position not finite at u={u:.3} v={v:.3}: {p:?}");
            }
        }
    }

    #[test]
    fn metric_is_positive_definite() {
        let kb = KleinBottle::default();
        for ui in 1..7u32 {
            for vi in 1..7u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * TAU / 8.0;
                let g = kb.metric(u, v);
                let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
                assert!(g[0][0] > 0.0, "g_00 ≤ 0 at u={u:.3} v={v:.3}");
                assert!(det > 0.0, "det(g) ≤ 0 at u={u:.3} v={v:.3}");
            }
        }
    }

    #[test]
    fn christoffel_is_finite() {
        let kb = KleinBottle::default();
        let gamma = kb.christoffel(0.5, 0.5);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(gamma[k][i][j].is_finite(),
                        "Γ^{k}_{i}{j} not finite: {}", gamma[k][i][j]);
                }
            }
        }
    }

    #[test]
    fn christoffel_is_symmetric_in_lower_indices() {
        let kb = KleinBottle::default();
        let gamma = kb.christoffel(1.0, 1.0);
        for k in 0..2 {
            assert!((gamma[k][0][1] - gamma[k][1][0]).abs() < 1e-4,
                "Γ^{k}_01 != Γ^{k}_10");
        }
    }

    #[test]
    fn normal_is_unit() {
        let kb = KleinBottle::default();
        let n = kb.normal(1.0, 1.0);
        assert!((n.length() - 1.0).abs() < 1e-4, "normal not unit: {}", n.length());
    }

    #[test]
    fn wrap_is_periodic() {
        let kb = KleinBottle::default();
        let (u, v) = kb.wrap(TAU + 0.3, -0.5);
        assert!((0.0..TAU).contains(&u));
        assert!((0.0..TAU).contains(&v));
    }

    #[test]
    fn mesh_vertex_count() {
        let kb = KleinBottle::default();
        let (verts, indices) = kb.mesh_vertices(8, 8);
        assert_eq!(verts.len(), 9 * 9);
        assert_eq!(indices.len(), 8 * 8 * 6);
    }
}
