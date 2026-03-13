use super::Surface;
use glam::Vec3;
use std::f32::consts::{TAU, PI};

/// Unit sphere: u ∈ [0, 2π), v ∈ (0, π)
/// x = sin(v) cos(u), y = sin(v) sin(u), z = cos(v)
pub struct Sphere {
    pub radius: f32,
}

impl Sphere {
    pub fn new(radius: f32) -> Self { Self { radius } }

    fn d_du(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(-self.radius * v.sin() * u.sin(),
                   self.radius * v.sin() * u.cos(),
                   0.0)
    }
    fn d_dv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new( self.radius * v.cos() * u.cos(),
                   self.radius * v.cos() * u.sin(),
                  -self.radius * v.sin())
    }
}

impl Surface for Sphere {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(self.radius * v.sin() * u.cos(),
                  self.radius * v.sin() * u.sin(),
                  self.radius * v.cos())
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let e1 = self.d_du(u, v);
        let e2 = self.d_dv(u, v);
        [[e1.dot(e1), e1.dot(e2)], [e1.dot(e2), e2.dot(e2)]]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        // g_00 = r² sin²v, g_11 = r², g_01 = 0
        // Γ^0_01 = Γ^0_10 = cos(v)/sin(v)
        // Γ^1_00 = -sin(v)cos(v)
        let sv = v.sin();
        let cv = v.cos();
        let gamma_0_01 = if sv.abs() > 1e-6 { cv / sv } else { 0.0 };
        let gamma_1_00 = -sv * cv;
        [
            [[0.0, gamma_0_01], [gamma_0_01, 0.0]],
            [[gamma_1_00, 0.0], [0.0, 0.0]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        let u = u.rem_euclid(TAU);
        let v = v.clamp(0.01, PI - 0.01);
        (u, v)
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        self.position(u, v).normalize()
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        (rng.gen_range(0.0..TAU), rng.gen_range(0.1..PI - 0.1))
    }

    fn random_tangent(&self, _u: f32, _v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use rand::Rng;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let speed = 0.5f32;
        (angle.cos() * speed, angle.sin() * speed)
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        for i in 0..=u_steps {
            for j in 0..=v_steps {
                let u = (i as f32 / u_steps as f32) * TAU;
                let v = 0.01 + (j as f32 / v_steps as f32) * (PI - 0.02);
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
