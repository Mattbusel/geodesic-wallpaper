//! Hyperboloid surface parameterization with analytic Christoffel symbols.
//!
//! The one-sheeted hyperboloid is a doubly-ruled quadric surface. This
//! implementation uses a hyperbolic parameterization.

use super::Surface;
use glam::Vec3;
use std::f32::consts::TAU;

/// One-sheeted hyperboloid with semi-axes `a` and `b`.
///
/// # Parameterization
/// - `u ∈ [0, 2π)` — azimuthal angle
/// - `v ∈ [-2, 2]` — hyperbolic latitude
///
/// ```text
/// x = a · cosh(v) · cos(u)
/// y = a · cosh(v) · sin(u)
/// z = b · sinh(v)
/// ```
///
/// Metric: `g_uu = a²·cosh²(v)`, `g_uv = 0`,
/// `g_vv = a²·sinh²(v) + b²·cosh²(v)`.
pub struct Hyperboloid {
    /// Semi-axis controlling the waist radius.
    pub a: f32,
    /// Semi-axis controlling the axial extent.
    pub b: f32,
}

impl Hyperboloid {
    /// Construct a hyperboloid with semi-axes `a` and `b`.
    pub fn new(a: f32, b: f32) -> Self {
        Self { a, b }
    }

    /// First partial derivative `∂φ/∂u` at `(u, v)`.
    fn d_du(&self, u: f32, v: f32) -> Vec3 {
        let ch = v.cosh();
        Vec3::new(-self.a * ch * u.sin(), self.a * ch * u.cos(), 0.0)
    }

    /// First partial derivative `∂φ/∂v` at `(u, v)`.
    fn d_dv(&self, u: f32, v: f32) -> Vec3 {
        let sh = v.sinh();
        let ch = v.cosh();
        Vec3::new(self.a * sh * u.cos(), self.a * sh * u.sin(), self.b * ch)
    }
}

impl Surface for Hyperboloid {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            self.a * v.cosh() * u.cos(),
            self.a * v.cosh() * u.sin(),
            self.b * v.sinh(),
        )
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, _u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // g_uu = a²·cosh²(v), g_vv = a²·sinh²(v) + b²·cosh²(v), g_uv = 0
        //
        // g^uu = 1 / (a²·cosh²(v))
        // g^vv = 1 / (a²·sinh²(v) + b²·cosh²(v))
        //
        // Non-zero Christoffel symbols:
        //   Γ^u_uv = (1/2) g^uu ∂_v g_uu
        //           = (1/2)(1/(a²cosh²v))(2a²coshv·sinhv)
        //           = tanh(v)
        //
        //   Γ^v_uu = -(1/2) g^vv ∂_v g_uu
        //           = -(1/2)(1/(a²sinh²v+b²cosh²v))(2a²coshv·sinhv)
        //           = -a²·sinh(v)·cosh(v) / (a²·sinh²(v) + b²·cosh²(v))
        //
        //   Γ^v_vv = (1/2) g^vv ∂_v g_vv
        //   ∂_v g_vv = 2a²·sinh(v)·cosh(v) + 2b²·cosh(v)·sinh(v)
        //            = 2(a²+b²)·sinh(v)·cosh(v)
        //   Γ^v_vv = (a²+b²)·sinh(v)·cosh(v) / (a²·sinh²(v) + b²·cosh²(v))
        let sh = v.sinh();
        let ch = v.cosh();
        let a2 = self.a * self.a;
        let b2 = self.b * self.b;
        let g_vv = a2 * sh * sh + b2 * ch * ch;
        let g_vv_inv = 1.0 / g_vv.max(1e-8);

        let gamma_u_uv = v.tanh();
        let gamma_v_uu = -a2 * sh * ch * g_vv_inv;
        let gamma_v_vv = (a2 + b2) * sh * ch * g_vv_inv;

        [
            // k=0 (u): Γ^u_ij
            [[0.0, gamma_u_uv], [gamma_u_uv, 0.0]],
            // k=1 (v): Γ^v_ij
            [[gamma_v_uu, 0.0], [0.0, gamma_v_vv]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        let u = u.rem_euclid(TAU);
        let v = v.clamp(-2.0, 2.0);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        e1.cross(e2).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(0.0..TAU), rng.gen_range(-2.0..2.0_f32))
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let g = self.metric(u, v);
        // Normalise so g_ij du^i du^j = 1.
        let du = angle.cos() / g[0][0].sqrt().max(1e-8);
        let dv = angle.sin() / g[1][1].sqrt().max(1e-8);
        (du, dv)
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=u_steps {
            for j in 0..=v_steps {
                let u = (i as f32 / u_steps as f32) * TAU;
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
    fn position_at_known_point() {
        // At u=0, v=0: x = a·cosh(0)·cos(0) = a, y = 0, z = b·sinh(0) = 0.
        let hyp = Hyperboloid::new(1.0, 1.0);
        let p = hyp.position(0.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-5, "x={}", p.x);
        assert!(p.y.abs() < 1e-5, "y={}", p.y);
        assert!(p.z.abs() < 1e-5, "z={}", p.z);
    }

    #[test]
    fn metric_is_diagonal() {
        let hyp = Hyperboloid::new(1.0, 1.0);
        let g = hyp.metric(0.5, 0.5);
        assert!((g[0][1]).abs() < 1e-4, "g_01 not zero: {}", g[0][1]);
    }

    #[test]
    fn metric_g_uu_formula() {
        // g_uu = a²·cosh²(v)
        let hyp = Hyperboloid::new(2.0, 1.0);
        let v = 0.5_f32;
        let g = hyp.metric(0.0, v);
        let expected = 4.0 * v.cosh() * v.cosh();
        assert!((g[0][0] - expected).abs() < 1e-4, "g_uu={} expected={}", g[0][0], expected);
    }

    #[test]
    fn christoffel_symmetry_ij() {
        let hyp = Hyperboloid::new(1.0, 1.0);
        let g = hyp.christoffel(0.4, 0.8);
        for k in 0..2 {
            assert!(
                (g[k][0][1] - g[k][1][0]).abs() < 1e-6,
                "Γ^{k}_01 != Γ^{k}_10"
            );
        }
    }

    #[test]
    fn wrap_clamps_v_and_wraps_u() {
        let hyp = Hyperboloid::new(1.0, 1.0);
        let (u, v) = hyp.wrap(TAU + 0.3, 5.0);
        assert!((0.0..TAU).contains(&u), "u={u}");
        assert!(v <= 2.0, "v not clamped: {v}");
        assert!(v >= -2.0, "v not clamped: {v}");
    }

    #[test]
    fn normal_is_unit() {
        let hyp = Hyperboloid::new(1.0, 1.0);
        let n = hyp.normal(1.0, 0.5);
        assert!(
            (n.length() - 1.0).abs() < 1e-5,
            "normal length={}",
            n.length()
        );
    }
}
