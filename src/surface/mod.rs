//! Surface abstractions for geodesic computation.
//!
//! Each surface implements the [`Surface`] trait, providing a parameterisation,
//! metric tensor, Christoffel symbols, and helpers needed by the RK4 integrator
//! and the mesh renderer.

pub mod boy_surface;
pub mod catenoid;
pub mod ellipsoid;
pub mod enneper;
pub mod helicoid;
pub mod hyperboloid;
pub mod hyperbolic_paraboloid;
pub mod klein_bottle;
pub mod saddle;
pub mod sphere;
pub mod torus;
pub mod torus_knot;

use glam::Vec3;

/// A smooth parameterised surface embedded in ℝ³.
///
/// The two surface parameters are conventionally called `u` and `v`. All
/// implementations must be `Send + Sync` so they can be shared across threads.
pub trait Surface: Send + Sync {
    /// Map the parameter pair `(u, v)` to a point in ℝ³.
    ///
    /// This is the embedding function `φ: U ⊂ ℝ² → ℝ³`.
    fn position(&self, u: f32, v: f32) -> Vec3;

    /// Compute the metric tensor `g_ij` at `(u, v)`.
    ///
    /// Returns a 2×2 symmetric matrix `[[g_00, g_01], [g_10, g_11]]` where
    /// each component is the inner product of the coordinate tangent vectors:
    /// `g_ij = ∂_i φ · ∂_j φ`.
    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2];

    /// Compute all Christoffel symbols of the second kind `Γ^k_ij` at `(u, v)`.
    ///
    /// The returned array has shape `[k][i][j]`, so `result[k][i][j]` is
    /// `Γ^k_ij`. For a 2-D surface this yields 2×2×2 = 8 values.
    ///
    /// Christoffel symbols are defined via
    /// `Γ^k_ij = ½ g^{kl} (∂_i g_{lj} + ∂_j g_{li} − ∂_l g_{ij})`.
    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2];

    /// Wrap or clamp `(u, v)` back into the valid parameter domain.
    ///
    /// For periodic surfaces (torus, sphere longitude) this applies
    /// `rem_euclid`; for bounded surfaces (saddle) it applies `clamp`.
    fn wrap(&self, u: f32, v: f32) -> (f32, f32);

    /// Compute the outward unit normal at `(u, v)`.
    ///
    /// For immersed surfaces this is `(∂_u φ × ∂_v φ) / |∂_u φ × ∂_v φ|`.
    fn normal(&self, u: f32, v: f32) -> Vec3;

    /// Sample a uniformly random valid parameter position `(u, v)`.
    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32);

    /// Sample a random unit-speed tangent vector `(du, dv)` at `(u, v)`.
    ///
    /// The returned velocity satisfies `g_ij du^i du^j ≈ 1` so that all
    /// geodesics start with the same speed regardless of surface curvature.
    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32);

    /// Generate a triangulated mesh for background rendering.
    ///
    /// Returns `(vertices, indices)` where each vertex is `[x, y, z]` and
    /// indices are triples forming triangles. `u_steps × v_steps` quads are
    /// produced and split into two triangles each.
    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>);
}
