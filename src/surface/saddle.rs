use super::Surface;
use glam::Vec3;

/// Hyperbolic paraboloid (saddle): z = (x² - y²) / a
/// u ∈ [-2, 2], v ∈ [-2, 2]
pub struct Saddle {
    pub scale: f32,
}

impl Saddle {
    pub fn new(scale: f32) -> Self { Self { scale } }

    fn embed(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(u, v, (u * u - v * v) / self.scale)
    }

    fn d_du(&self, u: f32, _v: f32) -> Vec3 {
        Vec3::new(1.0, 0.0, 2.0 * u / self.scale)
    }

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
