//! Catenoid minimal surface parameterization with analytic Christoffel symbols.

use super::Surface;
use glam::Vec3;
use rand::Rng;
use std::f32::consts::TAU;

/// The catenoid — the surface of revolution of the catenary `r = cosh(z)`.
///
/// # Parameterization
/// - `u ∈ [0, 2π)` — azimuth around the axis of symmetry
/// - `v ∈ [−v_max, v_max]` — height (clamped)
///
/// ```text
/// x = cosh(v) · cos(u)
/// y = cosh(v) · sin(u)
/// z = v
/// ```
///
/// The catenoid is a minimal surface (mean curvature zero everywhere) and
/// is locally isometric to the helicoid.
pub struct Catenoid {
    /// Half-height of the visible domain.
    pub v_max: f32,
}

impl Catenoid {
    /// Create a catenoid with the given half-height.
    ///
    /// The default half-height `1.5` shows the characteristic waist shape.
    pub fn new(v_max: f32) -> Self {
        Self {
            v_max: v_max.max(0.1),
        }
    }
}

impl Surface for Catenoid {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        let ch = v.cosh();
        Vec3::new(ch * u.cos(), ch * u.sin(), v)
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        // ∂φ/∂u = (−cosh(v)·sin(u), cosh(v)·cos(u), 0)  → |eu|² = cosh²(v)
        // ∂φ/∂v = (sinh(v)·cos(u), sinh(v)·sin(u), 1)   → |ev|² = sinh²(v) + 1 = cosh²(v)
        // eu·ev = 0  (orthogonal parameterization)
        let _ = u;
        let ch2 = v.cosh() * v.cosh();
        [[ch2, 0.0], [0.0, ch2]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Orthogonal metric: g_uu = g_vv = cosh²(v), g_uv = 0.
        // Non-zero Christoffel symbols:
        //   Γ^u_{uv} = Γ^u_{vu} = tanh(v)
        //   Γ^v_{uu} = −sinh(v)·cosh(v)
        //   Γ^v_{vv} = tanh(v)
        let _ = u;
        let tanh_v = v.tanh();
        let sinh_v = v.sinh();
        let cosh_v = v.cosh();
        [
            // k=0 (u component)
            [[0.0, tanh_v], [tanh_v, 0.0]],
            // k=1 (v component)
            [[-sinh_v * cosh_v, 0.0], [0.0, tanh_v]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        (u.rem_euclid(TAU), v.clamp(-self.v_max, self.v_max))
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        // N = ∂u × ∂v (normalised)
        let eu = Vec3::new(-v.cosh() * u.sin(), v.cosh() * u.cos(), 0.0);
        let ev = Vec3::new(v.sinh() * u.cos(), v.sinh() * u.sin(), 1.0);
        let n = eu.cross(ev);
        let len = n.length();
        if len > 1e-12 { n / len } else { Vec3::Z }
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        (
            rng.gen_range(0.0..TAU),
            rng.gen_range(-self.v_max..self.v_max),
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
        for vi in 0..=v_steps {
            for ui in 0..=u_steps {
                let u = TAU * ui as f32 / u_steps as f32;
                let v = -self.v_max + 2.0 * self.v_max * vi as f32 / v_steps as f32;
                let p = self.position(u, v);
                verts.push([p.x, p.y, p.z]);
            }
        }
        let cols = u_steps + 1;
        for vi in 0..v_steps {
            for ui in 0..u_steps {
                let a = vi * cols + ui;
                let b = a + 1;
                let c = a + cols;
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
    fn catenoid_metric_is_conformal() {
        // The catenoid has a conformal metric: g_uu = g_vv, g_uv = 0.
        let c = Catenoid::new(1.5);
        for v in [0.0f32, 0.5, 1.0, -0.7] {
            let g = c.metric(0.3, v);
            assert!((g[0][0] - g[1][1]).abs() < 1e-5, "not conformal at v={v}");
            assert!(g[0][1].abs() < 1e-5, "g_uv != 0 at v={v}");
        }
    }

    #[test]
    fn catenoid_christoffel_symmetric() {
        let c = Catenoid::new(1.5);
        let g = c.christoffel(0.3, 0.7);
        for k in 0..2 {
            assert!(
                (g[k][0][1] - g[k][1][0]).abs() < 1e-5,
                "Γ^{k} not symmetric: {} vs {}",
                g[k][0][1],
                g[k][1][0]
            );
        }
    }

    #[test]
    fn catenoid_position_at_waist() {
        // At v=0, cosh(0) = 1, so position = (cos(u), sin(u), 0) — unit circle in XY.
        let c = Catenoid::new(1.5);
        let p = c.position(0.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-6, "waist x at u=0: {}", p.x);
        assert!(p.y.abs() < 1e-6);
        assert!(p.z.abs() < 1e-6);
    }
}
