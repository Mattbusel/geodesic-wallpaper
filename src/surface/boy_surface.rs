//! Boy's surface — a non-orientable immersion of the real projective plane RP² in ℝ³.
//!
//! Boy's surface is the unique regular immersion of RP² in ℝ³ with 3-fold
//! symmetry.  We use the parametric form due to Robert Bryant (1987), expressed
//! in terms of the Veronese map, which gives a particularly clean formula:
//!
//! Given `z = e^{iu}·tan(v/2)` (complex stereographic parameter with
//! `u ∈ [0, 2π)`, `v ∈ [0, π/2]`) the embedding is:
//!
//! ```text
//! g1 = − (3/2) Im[z(1 − z^4)] / D
//! g2 =   (3/2) Re[z(1 + z^4)] / D
//! g3 = Im[1 + z^6] / D − (1/2)
//! D  = Re[(1 + z^6)] + √5 · Im[z^3]
//! ```
//!
//! In practice we use a simpler real-variable form due to Apéry:
//!
//! ```text
//! x = (sqrt(2) cos²v cos(2u) + cos u sin(2v)) / (2 − sqrt(2) sin(3u) sin(2v))
//! y = (sqrt(2) cos²v sin(2u) − sin u sin(2v)) / (2 − sqrt(2) sin(3u) sin(2v))
//! z = (3 cos²v)                               / (2 − sqrt(2) sin(3u) sin(2v))
//! ```
//!
//! with `u ∈ [0, π)` and `v ∈ [0, π/2]`.
//!
//! Christoffel symbols are computed via numerical finite differences.

use super::Surface;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::{PI, SQRT_2, TAU};

/// Non-orientable immersion of RP² in ℝ³ with 3-fold symmetry.
///
/// # Parameterization
/// - `u ∈ [0, π)` — azimuthal angle
/// - `v ∈ [0, π/2]` — polar angle
///
/// The surface self-intersects at its triple point but is otherwise a valid
/// smooth immersion of the real projective plane.
pub struct BoySurface {
    /// Overall scale factor.  Default: `1.0`.
    pub scale: f32,
}

impl BoySurface {
    /// Construct a Boy surface with the given scale.
    pub fn new(scale: f32) -> Self {
        Self {
            scale: scale.max(0.1),
        }
    }

    fn embed(&self, u: f32, v: f32) -> Vec3 {
        let (su, cu) = u.sin_cos();
        let (sv, cv) = v.sin_cos();
        let s3u = (3.0 * u).sin();
        let s2v = (2.0 * v).sin();
        let c2u = (2.0 * u).cos();
        let s2u = (2.0 * u).sin();

        let denom = 2.0 - SQRT_2 * s3u * s2v;
        let safe_d = if denom.abs() > 1e-10 { denom } else { 1e-10 };

        let x = (SQRT_2 * cv * cv * c2u + cu * s2v) / safe_d;
        let y = (SQRT_2 * cv * cv * s2u - su * s2v) / safe_d;
        let z = 3.0 * cv * cv / safe_d;

        Vec3::new(x * self.scale, y * self.scale, z * self.scale)
    }

    fn du(&self, u: f32, v: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.embed(u + H, v) - self.embed(u - H, v)) * (0.5 / H)
    }

    fn dv(&self, u: f32, v: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.embed(u, v + H) - self.embed(u, v - H)) * (0.5 / H)
    }
}

impl Default for BoySurface {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Surface for BoySurface {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        self.embed(u, v)
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        [[eu.dot(eu), eu.dot(ev)], [eu.dot(ev), ev.dot(ev)]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        const H: f32 = 1e-3;
        let g = self.metric(u, v);
        let det = g[0][0] * g[1][1] - g[0][1] * g[0][1];
        let inv_det = if det.abs() > 1e-12 { 1.0 / det } else { 0.0 };
        let gi = [
            [g[1][1] * inv_det, -g[0][1] * inv_det],
            [-g[0][1] * inv_det, g[0][0] * inv_det],
        ];

        let metric_uh = self.metric(u + H, v);
        let metric_ul = self.metric(u - H, v);
        let metric_vh = self.metric(u, v + H);
        let metric_vl = self.metric(u, v - H);

        // dg[i][j][d] = ∂_d g_{ij}  (d=0 → ∂u, d=1 → ∂v)
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

        let mut gamma = [[[0.0f32; 2]; 2]; 2];
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    let mut s = 0.0f32;
                    for l in 0..2 {
                        s += gi[k][l] * (dg[l][j][i] + dg[l][i][j] - dg[i][j][l]);
                    }
                    gamma[k][i][j] = 0.5 * s;
                }
            }
        }
        gamma
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        // u is periodic over [0, π); v is clamped to [0, π/2].
        let u = u.rem_euclid(PI);
        let v = v.clamp(0.0, PI * 0.5);
        (u, v)
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
        // u in [0, π), v in [0, π/2].
        (rng.gen_range(0.0..PI), rng.gen_range(0.0..(PI * 0.5)))
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
                let u = (i as f32 / u_steps as f32) * PI;
                let v = (j as f32 / v_steps as f32) * PI * 0.5;
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
        let boy = BoySurface::default();
        for ui in 0..8u32 {
            for vi in 0..4u32 {
                let u = ui as f32 * PI / 8.0;
                let v = vi as f32 * PI / 8.0;
                let p = boy.position(u, v);
                assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite(),
                    "position not finite at u={u:.3} v={v:.3}: {p:?}");
            }
        }
    }

    #[test]
    fn metric_is_positive_definite() {
        let boy = BoySurface::default();
        // Test away from the degenerate pole v=0.
        for ui in 1..7u32 {
            for vi in 1..3u32 {
                let u = ui as f32 * PI / 8.0;
                let v = vi as f32 * PI / 8.0;
                let g = boy.metric(u, v);
                let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
                assert!(g[0][0] > 0.0, "g_00 ≤ 0 at u={u:.3} v={v:.3}");
                assert!(det > 0.0, "det(g) ≤ 0 at u={u:.3} v={v:.3}: det={det}");
            }
        }
    }

    #[test]
    fn christoffel_is_finite() {
        let boy = BoySurface::default();
        let gamma = boy.christoffel(0.8, 0.6);
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
        let boy = BoySurface::default();
        let gamma = boy.christoffel(1.0, 0.5);
        for k in 0..2 {
            assert!((gamma[k][0][1] - gamma[k][1][0]).abs() < 1e-3,
                "Γ^{k}_01 != Γ^{k}_10");
        }
    }

    #[test]
    fn normal_is_unit() {
        let boy = BoySurface::default();
        let n = boy.normal(1.0, 0.5);
        assert!((n.length() - 1.0).abs() < 1e-4, "normal not unit: {}", n.length());
    }

    #[test]
    fn wrap_clamps_v() {
        let boy = BoySurface::default();
        let (u, v) = boy.wrap(3.5, 2.0);
        assert!(u < PI, "u out of range: {u}");
        assert!(v <= PI * 0.5, "v out of range: {v}");
    }
}
