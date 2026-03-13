use super::Surface;
use glam::Vec3;
use std::f32::consts::TAU;

/// Torus with major radius R (center to tube center) and minor radius r (tube radius).
/// Parameterization: u ∈ [0, 2π), v ∈ [0, 2π)
/// x = (R + r cos v) cos u
/// y = (R + r cos v) sin u
/// z = r sin v
pub struct Torus {
    pub big_r: f32,
    pub small_r: f32,
}

impl Torus {
    pub fn new(big_r: f32, small_r: f32) -> Self {
        Self { big_r, small_r }
    }

    /// Partial derivatives of the embedding
    fn d_du(&self, u: f32, v: f32) -> Vec3 {
        let r = self.big_r + self.small_r * v.cos();
        Vec3::new(-r * u.sin(), r * u.cos(), 0.0)
    }

    fn d_dv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            -self.small_r * v.sin() * u.cos(),
            -self.small_r * v.sin() * u.sin(),
            self.small_r * v.cos(),
        )
    }

    fn d2_du2(&self, u: f32, v: f32) -> Vec3 {
        let r = self.big_r + self.small_r * v.cos();
        Vec3::new(-r * u.cos(), -r * u.sin(), 0.0)
    }

    fn d2_dv2(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            -self.small_r * v.cos() * u.cos(),
            -self.small_r * v.cos() * u.sin(),
            -self.small_r * v.sin(),
        )
    }

    fn d2_dudv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            self.small_r * v.sin() * u.sin(),
            -self.small_r * v.sin() * u.cos(),
            0.0,
        )
    }

    /// Compute metric and its inverse
    fn metric_and_inv(&self, u: f32, v: f32) -> ([[f32; 2]; 2], [[f32; 2]; 2]) {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        let g00 = e1.dot(e1);
        let g01 = e1.dot(e2);
        let g11 = e2.dot(e2);
        let g = [[g00, g01], [g01, g11]];
        let det = g00 * g11 - g01 * g01;
        let inv = [[g11 / det, -g01 / det], [-g01 / det, g00 / det]];
        (g, inv)
    }
}

impl Surface for Torus {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        let r = self.big_r + self.small_r * v.cos();
        Vec3::new(r * u.cos(), r * u.sin(), self.small_r * v.sin())
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // Γ^k_ij = (1/2) g^{kl} (∂_i g_{lj} + ∂_j g_{li} - ∂_l g_{ij})
        // For torus, g_01 = 0 everywhere (orthogonal parameterization)
        // g_00 = (R + r cos v)^2,  g_11 = r^2
        // ∂_u g_00 = 0,  ∂_v g_00 = -2(R + r cos v) r sin v
        // ∂_u g_11 = 0,  ∂_v g_11 = 0
        let f = self.big_r + self.small_r * v.cos();
        let df_dv = -self.small_r * v.sin();
        let g00 = f * f;
        let g11 = self.small_r * self.small_r;

        // Non-zero Christoffels for orthogonal parameterization:
        // Γ^0_01 = Γ^0_10 = (∂_v g_00) / (2 g_00) = df_dv / f
        // Γ^1_00 = -(∂_v g_00) / (2 g_11) = -f df_dv / r^2
        let gamma_0_01 = df_dv / f;
        let gamma_1_00 = -f * df_dv / g11;

        [
            // k=0: Γ^0_ij
            [[0.0, gamma_0_01], [gamma_0_01, 0.0]],
            // k=1: Γ^1_ij
            [[gamma_1_00, 0.0], [0.0, 0.0]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        let u = u.rem_euclid(TAU);
        let v = v.rem_euclid(TAU);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        e1.cross(e2).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(0.0..TAU), rng.gen_range(0.0..TAU))
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let (g, _) = self.metric_and_inv(u, v);
        // Normalize so g_ij du^i du^j = 1
        let speed = 1.0;
        let du = angle.cos() * speed / g[0][0].sqrt();
        let dv = angle.sin() * speed / g[1][1].sqrt();
        (du, dv)
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
