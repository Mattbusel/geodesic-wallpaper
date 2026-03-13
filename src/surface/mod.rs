pub mod torus;
pub mod sphere;
pub mod saddle;

use glam::Vec3;

/// A parameterized surface in 3D.
/// u, v are the two surface parameters.
pub trait Surface: Send + Sync {
    /// Embedding: (u,v) -> point in R^3
    fn position(&self, u: f32, v: f32) -> Vec3;

    /// Metric tensor components g_ij at (u,v)
    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2];

    /// Christoffel symbols Γ^k_ij at (u,v)
    /// Returns [[[Γ^0_00, Γ^0_01],[Γ^0_10, Γ^0_11]],
    ///          [[Γ^1_00, Γ^1_01],[Γ^1_10, Γ^1_11]]]
    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2];

    /// Domain wrapping: clamp or wrap u,v to valid range
    fn wrap(&self, u: f32, v: f32) -> (f32, f32);

    /// Normal vector at (u,v)
    fn normal(&self, u: f32, v: f32) -> Vec3;

    /// Sample a random valid (u,v) position
    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32);

    /// Sample a random unit tangent vector (du, dv) for geodesic initial condition
    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32);

    /// Generate mesh vertices for background rendering
    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>);
}
