//! Trefoil knot tube — a torus-like tube wrapped around the trefoil knot curve.
//!
//! The trefoil knot is the simplest non-trivial knot. Wrapping a circular tube
//! around it creates a surface embedded in ℝ³ with non-trivial topology:
//! geodesics spiral around the tube while the tube itself winds three times
//! around the central axis, producing stunning visual trajectories.
//!
//! # Parameterisation
//!
//! The knot centre-line is:
//!
//! ```text
//! C(t) = ( (2 + cos(3t))·cos(2t),
//!          (2 + cos(3t))·sin(2t),
//!          sin(3t) )    for t ∈ [0, 2π)
//! ```
//!
//! A circular tube of radius `r` is swept along `C(t)` using the Frenet-Serret
//! frame `{T, N, B}`:
//!
//! ```text
//! P(t, φ) = C(t) + r · (cos(φ)·N(t) + sin(φ)·B(t))
//! ```
//!
//! Both derivatives `∂P/∂t` and `∂P/∂φ` and all required Christoffel symbols
//! are computed using central finite differences (step h = 1e-4).  This avoids
//! the very lengthy analytic expressions for the Frenet frame derivatives while
//! maintaining sufficient accuracy for the RK4 integrator.
//!
//! # Topology
//!
//! The surface is a torus in the topological sense, but knotted in ℝ³. Its
//! total turning number causes any "straight" geodesic (φ = const, t increasing)
//! to precess around the tube, tracing a helical path across all three lobes of
//! the knot before almost closing — the visual highlight of this surface.

use super::Surface;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::TAU;

/// Tube of circular cross-section swept along the trefoil knot curve.
///
/// Parameters:
/// - `t ∈ [0, 2π)` — position along the knot
/// - `phi ∈ [0, 2π)` — angle around the tube cross-section
pub struct TrefoilTube {
    /// Radius of the tube cross-section.
    ///
    /// Must be small enough that the tube does not self-intersect.
    /// The knot has a minimum half-distance of ~0.5 between strands, so
    /// values in the range `0.1..0.35` are recommended.
    pub tube_radius: f32,
    /// Uniform scale applied to the whole surface.
    ///
    /// Default: `0.8` (fits nicely in a scene alongside other surfaces).
    pub scale: f32,
}

impl TrefoilTube {
    /// Construct a trefoil tube with the given tube radius and scale.
    pub fn new(tube_radius: f32, scale: f32) -> Self {
        Self {
            tube_radius: tube_radius.clamp(0.05, 0.4),
            scale: scale.max(0.1),
        }
    }

    /// Evaluate the trefoil knot centre-line at parameter `t`.
    #[inline]
    fn knot(t: f32) -> Vec3 {
        Vec3::new(
            (2.0 + (3.0 * t).cos()) * (2.0 * t).cos(),
            (2.0 + (3.0 * t).cos()) * (2.0 * t).sin(),
            (3.0 * t).sin(),
        )
    }

    /// Compute the Frenet-Serret frame `(T, N, B)` at `t` numerically.
    ///
    /// Returns `(tangent, principal_normal, binormal)`.  All three vectors are
    /// unit length.  If the curvature is zero (straight segment) the normal
    /// defaults to an arbitrary perpendicular direction.
    fn frenet(t: f32) -> (Vec3, Vec3, Vec3) {
        const H: f32 = 1e-4;
        // First derivative → tangent
        let c_fwd = Self::knot(t + H);
        let c_bwd = Self::knot(t - H);
        let tangent = (c_fwd - c_bwd) * (0.5 / H);
        let tangent_len = tangent.length();
        let t_hat = if tangent_len > 1e-10 {
            tangent / tangent_len
        } else {
            Vec3::X
        };

        // Second derivative → normal direction
        let c_mid = Self::knot(t);
        let accel = (c_fwd - 2.0 * c_mid + c_bwd) / (H * H);
        // Remove tangential component to get the curvature vector
        let accel_perp = accel - accel.dot(t_hat) * t_hat;
        let accel_len = accel_perp.length();
        let n_hat = if accel_len > 1e-10 {
            accel_perp / accel_len
        } else {
            // Degenerate: pick an arbitrary perpendicular
            t_hat.any_orthogonal_vector().normalize()
        };

        let b_hat = t_hat.cross(n_hat);
        (t_hat, n_hat, b_hat)
    }

    /// Evaluate the tube surface at `(t, phi)` without scaling.
    #[inline]
    fn position_raw(&self, t: f32, phi: f32) -> Vec3 {
        let c = Self::knot(t);
        let (_, n, b) = Self::frenet(t);
        c + self.tube_radius * (phi.cos() * n + phi.sin() * b)
    }

    /// Central-difference first derivative `∂P/∂t`.
    fn dp_dt(&self, t: f32, phi: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.position_raw(t + H, phi) - self.position_raw(t - H, phi)) * (0.5 / H)
    }

    /// Central-difference first derivative `∂P/∂phi`.
    fn dp_dphi(&self, t: f32, phi: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.position_raw(t, phi + H) - self.position_raw(t, phi - H)) * (0.5 / H)
    }

    /// Second derivative `∂²P/∂t²` (central differences).
    fn d2p_dt2(&self, t: f32, phi: f32) -> Vec3 {
        const H: f32 = 1e-4;
        let fwd = self.position_raw(t + H, phi);
        let mid = self.position_raw(t, phi);
        let bwd = self.position_raw(t - H, phi);
        (fwd - 2.0 * mid + bwd) / (H * H)
    }

    /// Second derivative `∂²P/∂phi²`.
    fn d2p_dphi2(&self, t: f32, phi: f32) -> Vec3 {
        const H: f32 = 1e-4;
        let fwd = self.position_raw(t, phi + H);
        let mid = self.position_raw(t, phi);
        let bwd = self.position_raw(t, phi - H);
        (fwd - 2.0 * mid + bwd) / (H * H)
    }

    /// Mixed second derivative `∂²P/∂t∂phi`.
    fn d2p_dtdphi(&self, t: f32, phi: f32) -> Vec3 {
        const H: f32 = 1e-4;
        (self.position_raw(t + H, phi + H)
            - self.position_raw(t + H, phi - H)
            - self.position_raw(t - H, phi + H)
            + self.position_raw(t - H, phi - H))
            / (4.0 * H * H)
    }
}

impl Default for TrefoilTube {
    fn default() -> Self {
        Self::new(0.25, 0.8)
    }
}

impl Surface for TrefoilTube {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        self.scale * self.position_raw(u, v)
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let s2 = self.scale * self.scale;
        let eu = self.dp_dt(u, v);
        let ev = self.dp_dphi(u, v);
        [
            [s2 * eu.dot(eu), s2 * eu.dot(ev)],
            [s2 * ev.dot(eu), s2 * ev.dot(ev)],
        ]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Scale-invariant (see pseudosphere comments).
        let eu = self.dp_dt(u, v);
        let ev = self.dp_dphi(u, v);
        let euu = self.d2p_dt2(u, v);
        let evv = self.d2p_dphi2(u, v);
        let euv = self.d2p_dtdphi(u, v);

        let g00 = eu.dot(eu);
        let g01 = eu.dot(ev);
        let g11 = ev.dot(ev);
        let det = g00 * g11 - g01 * g01;
        let inv_det = if det.abs() > 1e-12 { 1.0 / det } else { 0.0 };

        let gi00 = g11 * inv_det;
        let gi01 = -g01 * inv_det;
        let gi11 = g00 * inv_det;

        let sym = |d2: Vec3| -> (f32, f32) {
            let c0 = d2.dot(eu);
            let c1 = d2.dot(ev);
            (gi00 * c0 + gi01 * c1, gi01 * c0 + gi11 * c1)
        };

        let (g0_uu, g1_uu) = sym(euu);
        let (g0_vv, g1_vv) = sym(evv);
        let (g0_uv, g1_uv) = sym(euv);

        [
            [[g0_uu, g0_uv], [g0_uv, g0_vv]],
            [[g1_uu, g1_uv], [g1_uv, g1_vv]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        (u.rem_euclid(TAU), v.rem_euclid(TAU))
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let eu = self.dp_dt(u, v);
        let ev = self.dp_dphi(u, v);
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
            let u = (i as f32 / u_steps as f32) * TAU;
            for j in 0..=v_steps {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn surf() -> TrefoilTube {
        TrefoilTube::new(0.25, 0.8)
    }

    #[test]
    fn test_position_is_finite() {
        let s = surf();
        for &t in &[0.0f32, 0.5, 1.0, 2.0, 3.0, 5.0, TAU - 0.1] {
            for &phi in &[0.0f32, 1.0, 2.0, 3.0, TAU - 0.1] {
                let p = s.position(t, phi);
                assert!(p.x.is_finite());
                assert!(p.y.is_finite());
                assert!(p.z.is_finite());
            }
        }
    }

    #[test]
    fn test_metric_positive_definite() {
        let s = surf();
        let g = s.metric(1.0, 1.0);
        let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
        assert!(g[0][0] > 0.0, "g_uu not positive");
        assert!(det > 0.0, "metric not positive definite");
    }

    #[test]
    fn test_christoffel_no_nan() {
        let s = surf();
        let c = s.christoffel(1.0, 1.0);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(c[k][i][j].is_finite(), "Γ^{k}_{i}{j} is NaN/inf");
                }
            }
        }
    }

    #[test]
    fn test_christoffel_symmetric() {
        // Γ^k_{ij} must be symmetric in i,j
        let s = surf();
        let c = s.christoffel(1.5, 2.0);
        for k in 0..2 {
            assert!((c[k][0][1] - c[k][1][0]).abs() < 1e-3, "not symmetric");
        }
    }

    #[test]
    fn test_normal_is_unit() {
        let s = surf();
        let n = s.normal(1.0, 1.0);
        assert!((n.length() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_wrap_both_periodic() {
        let s = surf();
        let (u, v) = s.wrap(TAU + 1.0, TAU + 0.5);
        assert!((u - 1.0).abs() < 1e-5);
        assert!((v - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_mesh_vertex_count() {
        let s = surf();
        let (verts, indices) = s.mesh_vertices(12, 8);
        assert_eq!(verts.len(), 13 * 9);
        assert_eq!(indices.len(), 12 * 8 * 6);
    }

    #[test]
    fn test_knot_curve_period() {
        // C(0) and C(2π) should coincide (the knot is closed)
        let c0 = TrefoilTube::knot(0.0);
        let c2pi = TrefoilTube::knot(TAU);
        assert!(
            (c0 - c2pi).length() < 1e-4,
            "knot not closed: d={}",
            (c0 - c2pi).length()
        );
    }
}
