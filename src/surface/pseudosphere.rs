//! Pseudosphere (tractricoid) — a surface of constant negative Gaussian curvature.
//!
//! The pseudosphere is the classical model surface for hyperbolic geometry,
//! exhibiting constant curvature K = −1. Geodesics on the pseudosphere behave
//! very differently from those on the sphere: they diverge exponentially rather
//! than reconverging, making the pseudosphere visually striking as a wallpaper
//! surface — trails fan out and never return.
//!
//! # Parameterisation
//!
//! ```text
//! x(u, v) = sech(v) · cos(u)
//! y(u, v) = sech(v) · sin(u)
//! z(u, v) = v − tanh(v)
//! ```
//!
//! where `sech(v) = 1 / cosh(v)`.
//!
//! - `u ∈ [0, 2π)` — angular parameter (longitude around the axis)
//! - `v ∈ [0.05, 3.5]` — radial parameter (along the axis toward the cusp)
//!
//! The surface has a cuspidal singularity at `v = 0` (the sharp tip) and
//! flares out into a horn at large `v`. The domain is clamped away from the
//! cusp to keep the metric non-degenerate.
//!
//! # Metric
//!
//! The metric is orthogonal:
//! ```text
//! g_uu = sech²(v)      (circle radius at height v)
//! g_vv = tanh²(v)      (arc-length element along the axis)
//! g_uv = 0
//! ```
//!
//! # Christoffel symbols
//!
//! Because the metric is diagonal, the non-zero Christoffel symbols are:
//! ```text
//! Γ^u_uv = Γ^u_vu = −tanh(v)
//! Γ^v_uu = sech²(v) / tanh(v)
//! Γ^v_vv = sech²(v) / tanh(v)
//! ```
//! (all others zero)

use super::Surface;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::TAU;

/// Pseudosphere (tractricoid) — constant negative Gaussian curvature K = −1.
///
/// Visually resembles a "horn" or "bugle" shape. Geodesics exponentially
/// diverge, producing distinctive flaring trail patterns.
pub struct Pseudosphere {
    /// Scale factor applied uniformly to the embedding.
    ///
    /// Larger values make the surface appear bigger in the scene.
    /// Default: `1.5`.
    pub scale: f32,
    /// Maximum value of the `v` parameter.
    ///
    /// Controls how far along the horn the surface extends.
    /// Default: `3.0`.
    pub v_max: f32,
}

impl Pseudosphere {
    /// Construct a pseudosphere with the given scale and v-extent.
    pub fn new(scale: f32, v_max: f32) -> Self {
        Self {
            scale: scale.max(0.1),
            v_max: v_max.clamp(1.0, 5.0),
        }
    }

    // ── Internal derivatives (un-scaled) ────────────────────────────────────

    /// `∂φ/∂u` (un-scaled).
    #[inline]
    fn du_raw(u: f32, v: f32) -> Vec3 {
        let sech_v = 1.0 / v.cosh();
        Vec3::new(-u.sin() * sech_v, u.cos() * sech_v, 0.0)
    }

    /// `∂φ/∂v` (un-scaled).
    #[inline]
    fn dv_raw(u: f32, v: f32) -> Vec3 {
        let sech_v = 1.0 / v.cosh();
        let tanh_v = v.tanh();
        // ∂/∂v[sech(v)] = −sech(v)·tanh(v)
        Vec3::new(
            -u.cos() * tanh_v * sech_v,
            -u.sin() * tanh_v * sech_v,
            tanh_v * tanh_v, // ∂/∂v[v − tanh(v)] = 1 − sech²(v) = tanh²(v)
        )
    }

    /// `∂²φ/∂u²` (un-scaled).
    #[inline]
    fn d2u2_raw(u: f32, v: f32) -> Vec3 {
        let sech_v = 1.0 / v.cosh();
        Vec3::new(-u.cos() * sech_v, -u.sin() * sech_v, 0.0)
    }

    /// `∂²φ/∂v²` (un-scaled).
    ///
    /// Derived from:
    /// `∂²/∂v² [sech(v)·cos(u)] = −cos(u)·sech(v)·(2·sech²(v) − 1)`
    #[inline]
    fn d2v2_raw(u: f32, v: f32) -> Vec3 {
        let sech_v = 1.0 / v.cosh();
        let sech2 = sech_v * sech_v;
        let tanh_v = v.tanh();
        Vec3::new(
            -u.cos() * sech_v * (2.0 * sech2 - 1.0),
            -u.sin() * sech_v * (2.0 * sech2 - 1.0),
            2.0 * tanh_v * sech2, // ∂²/∂v² [v − tanh(v)] = 2·tanh(v)·sech²(v)
        )
    }

    /// `∂²φ/∂u∂v` (un-scaled).
    ///
    /// `∂/∂v[−sin(u)·sech(v)] = sin(u)·tanh(v)·sech(v)`
    #[inline]
    fn d2uv_raw(u: f32, v: f32) -> Vec3 {
        let sech_v = 1.0 / v.cosh();
        let tanh_v = v.tanh();
        Vec3::new(u.sin() * tanh_v * sech_v, -u.cos() * tanh_v * sech_v, 0.0)
    }
}

impl Default for Pseudosphere {
    fn default() -> Self {
        Self::new(1.5, 3.0)
    }
}

impl Surface for Pseudosphere {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        let sech_v = 1.0 / v.cosh();
        self.scale * Vec3::new(u.cos() * sech_v, u.sin() * sech_v, v - v.tanh())
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        // Scale factors cancel when we use the dot-product formula:
        // g_ij = (scale · e_i) · (scale · e_j) = scale² · (e_i · e_j)
        let s2 = self.scale * self.scale;
        let eu = Self::du_raw(u, v);
        let ev = Self::dv_raw(u, v);
        [
            [s2 * eu.dot(eu), s2 * eu.dot(ev)],
            [s2 * ev.dot(eu), s2 * ev.dot(ev)],
        ]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Christoffel symbols are invariant under uniform scaling:
        // Γ^k_ij = g^{kl} < ∂²φ/∂xi∂xj, ∂φ/∂xl >
        // The scale cancels between the second derivative factor (scale¹) and
        // the g^{kl} factor (scale⁻²), leaving one power of scale that again
        // cancels with the first-derivative factor (scale¹). Net: scale-free.
        let eu = Self::du_raw(u, v);
        let ev = Self::dv_raw(u, v);
        let euu = Self::d2u2_raw(u, v);
        let evv = Self::d2v2_raw(u, v);
        let euv = Self::d2uv_raw(u, v);

        let g00 = eu.dot(eu);
        let g01 = eu.dot(ev);
        let g11 = ev.dot(ev);
        let det = g00 * g11 - g01 * g01;
        let inv_det = if det.abs() > 1e-12 { 1.0 / det } else { 0.0 };

        let gi00 = g11 * inv_det;
        let gi01 = -g01 * inv_det;
        let gi11 = g00 * inv_det;

        // Γ^k_{ij} = g^{kl} <∂²φ/∂xi∂xj, ∂φ/∂xl>
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
        let u = u.rem_euclid(TAU);
        let v = v.clamp(0.05, self.v_max);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let eu = Self::du_raw(u, v);
        let ev = Self::dv_raw(u, v);
        let n = eu.cross(ev);
        let len = n.length();
        if len > 1e-12 {
            n / len
        } else {
            Vec3::Y
        }
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        (
            rng.gen_range(0.0..TAU),
            rng.gen_range(0.1..self.v_max * 0.9),
        )
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
                let v = 0.05 + (j as f32 / v_steps as f32) * (self.v_max - 0.05);
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

    fn surf() -> Pseudosphere {
        Pseudosphere::new(1.5, 3.0)
    }

    #[test]
    fn test_position_on_axis_z_component() {
        // At u=0, v=1: x = sech(1)·cos(0) = sech(1), y=0, z = 1 − tanh(1)
        let p = surf().position(0.0, 1.0);
        let sech1 = 1.0_f32 / 1.0_f32.cosh();
        let tanh1 = 1.0_f32.tanh();
        assert!((p.x - 1.5 * sech1).abs() < 1e-5);
        assert!((p.z - 1.5 * (1.0 - tanh1)).abs() < 1e-5);
    }

    #[test]
    fn test_metric_is_diagonal() {
        let s = surf();
        let g = s.metric(0.5, 1.0);
        // g_uv should be (near) zero for the pseudosphere
        assert!(g[0][1].abs() < 1e-5, "g_uv = {} (expected ~0)", g[0][1]);
    }

    #[test]
    fn test_metric_diagonal_positive() {
        let s = surf();
        let g = s.metric(1.0, 1.5);
        assert!(g[0][0] > 0.0);
        assert!(g[1][1] > 0.0);
    }

    #[test]
    fn test_wrap_u_periodic() {
        let s = surf();
        let (u, _) = s.wrap(TAU + 0.5, 1.0);
        assert!((u - 0.5).abs() < 1e-5);
    }

    #[test]
    fn test_wrap_v_clamps_low() {
        let s = surf();
        let (_, v) = s.wrap(0.0, -1.0);
        assert!(v >= 0.05);
    }

    #[test]
    fn test_wrap_v_clamps_high() {
        let s = surf();
        let (_, v) = s.wrap(0.0, 100.0);
        assert!(v <= s.v_max);
    }

    #[test]
    fn test_normal_is_unit_length() {
        let s = surf();
        let n = s.normal(1.0, 1.0);
        assert!((n.length() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_christoffel_no_nan() {
        let s = surf();
        let c = s.christoffel(0.7, 1.2);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(c[k][i][j].is_finite(), "Γ^{k}_{i}{j} is NaN/inf");
                }
            }
        }
    }

    #[test]
    fn test_mesh_vertex_count() {
        let s = surf();
        let (verts, indices) = s.mesh_vertices(16, 8);
        assert_eq!(verts.len(), 17 * 9);
        assert_eq!(indices.len(), 16 * 8 * 6);
    }
}
