//! Torus knot surface — a tube swept around a (p, q) torus knot curve.
//!
//! A **torus knot** T(p, q) is a closed curve that winds `p` times around
//! the longitude of a torus and `q` times around its meridian.  We sweep a
//! circular disk of radius `r_tube` along the curve to obtain a surface.
//!
//! # Curve parameterization
//!
//! ```text
//! x_curve(t) = (R + r_knot · cos(q·t)) · cos(p·t)
//! y_curve(t) = (R + r_knot · cos(q·t)) · sin(p·t)
//! z_curve(t) =  r_knot · sin(q·t)
//! ```
//!
//! where `t ∈ [0, 2π)`.
//!
//! # Surface parameterization
//!
//! Given the curve tangent `T`, an arbitrary normal `N` (Bishop frame), and
//! binormal `B = T × N`, the swept surface is:
//!
//! ```text
//! φ(t, s) = curve(t) + r_tube · (cos(s) · N(t) + sin(s) · B(t))
//! ```
//!
//! with `s ∈ [0, 2π)`.
//!
//! Christoffel symbols are computed via numerical finite differences of the
//! metric tensor.

use super::Surface;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::TAU;

/// A tube swept around a (p, q) torus knot curve.
///
/// # Parameters
/// - `p` — longitudinal winding number (default: 2)
/// - `q` — meridional winding number (default: 3); `gcd(p,q) = 1` for a knot
/// - `big_r` — major radius of the underlying torus (default: 2.0)
/// - `r_knot` — minor radius of the underlying torus (default: 0.8)
/// - `r_tube` — tube radius of the swept surface (default: 0.15)
pub struct TorusKnot {
    /// Longitudinal winding number.
    pub p: i32,
    /// Meridional winding number.
    pub q: i32,
    /// Major radius of the background torus.
    pub big_r: f32,
    /// Minor radius of the background torus.
    pub r_knot: f32,
    /// Radius of the swept tube.
    pub r_tube: f32,
}

impl TorusKnot {
    /// Construct a torus knot surface.
    pub fn new(p: i32, q: i32, big_r: f32, r_knot: f32, r_tube: f32) -> Self {
        Self {
            p: p.abs().max(2),
            q: q.abs().max(1),
            big_r: big_r.max(0.5),
            r_knot: r_knot.max(0.1),
            r_tube: r_tube.max(0.01),
        }
    }

    /// Evaluate the knot curve at parameter `t`.
    fn curve(&self, t: f32) -> Vec3 {
        let pt = self.p as f32 * t;
        let qt = self.q as f32 * t;
        let r = self.big_r + self.r_knot * qt.cos();
        Vec3::new(r * pt.cos(), r * pt.sin(), self.r_knot * qt.sin())
    }

    /// Curve tangent (unnormalized) at `t`.
    fn curve_tangent(&self, t: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.curve(t + H) - self.curve(t - H)) * (0.5 / H)
    }

    /// Compute a Bishop-frame normal at `t` (not parallel-transported, but
    /// consistent for rendering by taking a stable cross-product with a fixed axis).
    fn frame(&self, t: f32) -> (Vec3, Vec3) {
        let tan = self.curve_tangent(t);
        let tan_n = if tan.length() > 1e-12 {
            tan.normalize()
        } else {
            Vec3::X
        };
        // Pick a reference vector not parallel to tangent.
        let up = if tan_n.dot(Vec3::Z).abs() < 0.9 {
            Vec3::Z
        } else {
            Vec3::X
        };
        let binorm = tan_n.cross(up).normalize();
        let normal = binorm.cross(tan_n).normalize();
        (normal, binorm)
    }

    fn embed(&self, t: f32, s: f32) -> Vec3 {
        let (norm, binorm) = self.frame(t);
        self.curve(t) + self.r_tube * (s.cos() * norm + s.sin() * binorm)
    }

    fn dt(&self, t: f32, s: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.embed(t + H, s) - self.embed(t - H, s)) * (0.5 / H)
    }

    fn ds(&self, t: f32, s: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.embed(t, s + H) - self.embed(t, s - H)) * (0.5 / H)
    }
}

impl Default for TorusKnot {
    fn default() -> Self {
        Self::new(2, 3, 2.0, 0.8, 0.15)
    }
}

impl Surface for TorusKnot {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        // u ↦ t (curve parameter), v ↦ s (tube angle)
        self.embed(u, v)
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let eu = self.dt(u, v);
        let ev = self.ds(u, v);
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

        let muh = self.metric(u + H, v);
        let mul = self.metric(u - H, v);
        let mvh = self.metric(u, v + H);
        let mvl = self.metric(u, v - H);

        let dg = [
            [
                [(muh[0][0] - mul[0][0]) * 0.5 / H, (mvh[0][0] - mvl[0][0]) * 0.5 / H],
                [(muh[0][1] - mul[0][1]) * 0.5 / H, (mvh[0][1] - mvl[0][1]) * 0.5 / H],
            ],
            [
                [(muh[1][0] - mul[1][0]) * 0.5 / H, (mvh[1][0] - mvl[1][0]) * 0.5 / H],
                [(muh[1][1] - mul[1][1]) * 0.5 / H, (mvh[1][1] - mvl[1][1]) * 0.5 / H],
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
        (u.rem_euclid(TAU), v.rem_euclid(TAU))
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let eu = self.dt(u, v);
        let ev = self.ds(u, v);
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
        let tk = TorusKnot::default();
        for ui in 0..8u32 {
            for vi in 0..8u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * TAU / 8.0;
                let p = tk.position(u, v);
                assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite(),
                    "position not finite at u={u:.3} v={v:.3}: {p:?}");
            }
        }
    }

    #[test]
    fn metric_is_positive_definite() {
        let tk = TorusKnot::default();
        for ui in 0..8u32 {
            for vi in 0..8u32 {
                let u = ui as f32 * TAU / 8.0;
                let v = vi as f32 * TAU / 8.0;
                let g = tk.metric(u, v);
                let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
                assert!(g[0][0] > 0.0, "g_00 ≤ 0 at u={u:.3} v={v:.3}: g_00={}", g[0][0]);
                assert!(det > 0.0, "det(g) ≤ 0 at u={u:.3} v={v:.3}: det={det}");
            }
        }
    }

    #[test]
    fn christoffel_is_finite() {
        let tk = TorusKnot::default();
        let gamma = tk.christoffel(1.0, 0.5);
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
        let tk = TorusKnot::default();
        let gamma = tk.christoffel(1.0, 1.0);
        for k in 0..2 {
            assert!((gamma[k][0][1] - gamma[k][1][0]).abs() < 1e-3,
                "Γ^{k}_01 != Γ^{k}_10");
        }
    }

    #[test]
    fn trefoil_knot_is_periodic() {
        // Trefoil: T(2,3).  The curve should return to start after t = 2π.
        let tk = TorusKnot::new(2, 3, 2.0, 0.8, 0.15);
        let p0 = tk.curve(0.0);
        let p1 = tk.curve(TAU);
        assert!((p0 - p1).length() < 1e-3,
            "Trefoil not periodic: {:?} vs {:?}", p0, p1);
    }

    #[test]
    fn normal_is_unit() {
        let tk = TorusKnot::default();
        let n = tk.normal(1.0, 1.0);
        assert!((n.length() - 1.0).abs() < 1e-4, "normal not unit: {}", n.length());
    }

    #[test]
    fn wrap_is_periodic() {
        let tk = TorusKnot::default();
        let (u, v) = tk.wrap(TAU + 0.5, -0.3);
        assert!((0.0..TAU).contains(&u));
        assert!((0.0..TAU).contains(&v));
    }

    #[test]
    fn different_knot_types_give_different_shapes() {
        let trefoil = TorusKnot::new(2, 3, 2.0, 0.8, 0.15);
        let cinquefoil = TorusKnot::new(2, 5, 2.0, 0.8, 0.15);
        let p_trefoil = trefoil.position(1.0, 1.0);
        let p_cinquefoil = cinquefoil.position(1.0, 1.0);
        assert!((p_trefoil - p_cinquefoil).length() > 0.01,
            "Different knot types should give different positions");
    }
}
