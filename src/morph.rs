//! Surface morphing — smooth interpolation between surface types.
//!
//! Provides a [`SurfaceMorph`] that blends between two [`Surface`] implementations
//! over a configurable duration. The blend parameter `t ∈ [0, 1]` is driven by an
//! easing curve (smooth-step by default) to give a visually pleasing transition.
//!
//! # Mesh blending
//!
//! Both surfaces are sampled on the same UV grid, and vertex positions are
//! linearly interpolated:
//!
//! ```text
//! P(u, v, t) = (1 − t) · surface_a.position(u, v) + t · surface_b.position(u, v)
//! ```
//!
//! Christoffel symbols and the metric are also blended (numerically) so that
//! geodesics computed on the morphing surface are geometrically consistent.
//!
//! # GLSL integration
//!
//! The `morph_t` uniform is forwarded to the vertex shader which can use
//! `mix(pos_a, pos_b, morph_t)` for GPU-side interpolation when both meshes are
//! uploaded as separate vertex buffers.

#![allow(dead_code)]

use glam::Vec3;
use rand::RngCore;

use crate::surface::Surface;

// ── Easing ────────────────────────────────────────────────────────────────────

/// Smooth-step easing: S(t) = t² (3 − 2t).
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Smoother-step easing: S(t) = t³ (6t² − 15t + 10).
#[inline]
pub fn smootherstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

// ── Morph state ────────────────────────────────────────────────────────────────

/// Progress of a surface morph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MorphState {
    /// Not morphing; showing surface A entirely.
    AtA,
    /// Morphing from A → B.
    MorphingAToB {
        /// Raw (uneased) blend progress [0, 1].
        t: f32,
    },
    /// Not morphing; showing surface B entirely.
    AtB,
    /// Morphing from B → A.
    MorphingBToA { t: f32 },
}

// ── Surface morph ─────────────────────────────────────────────────────────────

/// Interpolates between two surfaces over time.
///
/// Drive with [`SurfaceMorph::tick`] each frame and read the current blend
/// parameter via [`SurfaceMorph::blend_t`].  Implement GPU-side blending by
/// uploading both meshes and using the GLSL snippet:
///
/// ```glsl
/// vec3 pos = mix(pos_a, pos_b, morph_t);
/// ```
pub struct SurfaceMorph {
    pub surface_a: Box<dyn Surface>,
    pub surface_b: Box<dyn Surface>,
    /// Duration of a full A → B or B → A morph in seconds.
    pub duration_s: f32,
    state: MorphState,
    /// Accumulated progress in seconds.
    elapsed: f32,
}

impl SurfaceMorph {
    /// Create a new morph between `surface_a` and `surface_b`.
    pub fn new(surface_a: Box<dyn Surface>, surface_b: Box<dyn Surface>, duration_s: f32) -> Self {
        Self {
            surface_a,
            surface_b,
            duration_s: duration_s.max(0.01),
            state: MorphState::AtA,
            elapsed: 0.0,
        }
    }

    /// Advance the morph by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        match self.state {
            MorphState::AtA | MorphState::AtB => {} // no-op
            MorphState::MorphingAToB { ref mut t } => {
                self.elapsed += dt;
                *t = (self.elapsed / self.duration_s).min(1.0);
                if *t >= 1.0 {
                    self.state = MorphState::AtB;
                    self.elapsed = 0.0;
                }
            }
            MorphState::MorphingBToA { ref mut t } => {
                self.elapsed += dt;
                *t = (self.elapsed / self.duration_s).min(1.0);
                if *t >= 1.0 {
                    self.state = MorphState::AtA;
                    self.elapsed = 0.0;
                }
            }
        }
    }

    /// Start a morph from A to B (or reverse if already at B).
    pub fn start_morph(&mut self) {
        match self.state {
            MorphState::AtA => {
                self.state = MorphState::MorphingAToB { t: 0.0 };
                self.elapsed = 0.0;
            }
            MorphState::AtB => {
                self.state = MorphState::MorphingBToA { t: 0.0 };
                self.elapsed = 0.0;
            }
            _ => {} // already morphing
        }
    }

    /// The eased blend parameter in `[0, 1]`.
    ///
    /// 0 = fully surface A, 1 = fully surface B.
    pub fn blend_t(&self) -> f32 {
        match self.state {
            MorphState::AtA => 0.0,
            MorphState::AtB => 1.0,
            MorphState::MorphingAToB { t } => smootherstep(t),
            MorphState::MorphingBToA { t } => 1.0 - smootherstep(t),
        }
    }

    /// The raw (uneased) progress in `[0, 1]`.
    pub fn raw_t(&self) -> f32 {
        match self.state {
            MorphState::AtA => 0.0,
            MorphState::AtB => 1.0,
            MorphState::MorphingAToB { t } | MorphState::MorphingBToA { t } => t,
        }
    }

    pub fn state(&self) -> MorphState {
        self.state
    }

    pub fn is_morphing(&self) -> bool {
        matches!(
            self.state,
            MorphState::MorphingAToB { .. } | MorphState::MorphingBToA { .. }
        )
    }

    /// Build interpolated mesh vertices for the current blend parameter.
    ///
    /// Returns `(vertices, indices)` where vertices are blended between the
    /// two surface meshes.
    pub fn blended_mesh(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let t = self.blend_t();
        let (verts_a, indices) = self.surface_a.mesh_vertices(u_steps, v_steps);
        let (verts_b, _) = self.surface_b.mesh_vertices(u_steps, v_steps);

        let verts: Vec<[f32; 3]> = verts_a
            .iter()
            .zip(verts_b.iter())
            .map(|(&a, &b)| {
                [
                    a[0] * (1.0 - t) + b[0] * t,
                    a[1] * (1.0 - t) + b[1] * t,
                    a[2] * (1.0 - t) + b[2] * t,
                ]
            })
            .collect();

        (verts, indices)
    }
}

/// [`SurfaceMorph`] also implements [`Surface`] itself so it can be used
/// transparently wherever a `&dyn Surface` is expected.
impl Surface for SurfaceMorph {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        let t = self.blend_t();
        let pa = self.surface_a.position(u, v);
        let pb = self.surface_b.position(u, v);
        pa.lerp(pb, t)
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let t = self.blend_t();
        let ga = self.surface_a.metric(u, v);
        let gb = self.surface_b.metric(u, v);
        [
            [
                ga[0][0] * (1.0 - t) + gb[0][0] * t,
                ga[0][1] * (1.0 - t) + gb[0][1] * t,
            ],
            [
                ga[1][0] * (1.0 - t) + gb[1][0] * t,
                ga[1][1] * (1.0 - t) + gb[1][1] * t,
            ],
        ]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        let t = self.blend_t();
        let ca = self.surface_a.christoffel(u, v);
        let cb = self.surface_b.christoffel(u, v);
        let mut out = [[[0.0f32; 2]; 2]; 2];
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    out[k][i][j] = ca[k][i][j] * (1.0 - t) + cb[k][i][j] * t;
                }
            }
        }
        out
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        // Use surface A's wrapping when close to A, B's when close to B.
        let t = self.blend_t();
        if t < 0.5 {
            self.surface_a.wrap(u, v)
        } else {
            self.surface_b.wrap(u, v)
        }
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let t = self.blend_t();
        let na = self.surface_a.normal(u, v);
        let nb = self.surface_b.normal(u, v);
        na.lerp(nb, t).normalize()
    }

    fn random_position(&self, rng: &mut dyn RngCore) -> (f32, f32) {
        // Delegate to whichever surface is dominant.
        if self.blend_t() < 0.5 {
            self.surface_a.random_position(rng)
        } else {
            self.surface_b.random_position(rng)
        }
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn RngCore) -> (f32, f32) {
        if self.blend_t() < 0.5 {
            self.surface_a.random_tangent(u, v, rng)
        } else {
            self.surface_b.random_tangent(u, v, rng)
        }
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        self.blended_mesh(u_steps, v_steps)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::{sphere::Sphere, torus::Torus};

    fn make_morph() -> SurfaceMorph {
        SurfaceMorph::new(
            Box::new(Torus::new(2.0, 0.7)),
            Box::new(Sphere::new(1.5)),
            5.0,
        )
    }

    #[test]
    fn test_morph_starts_at_a() {
        let m = make_morph();
        assert_eq!(m.blend_t(), 0.0);
        assert_eq!(m.state(), MorphState::AtA);
    }

    #[test]
    fn test_morph_progresses_over_time() {
        let mut m = make_morph();
        m.start_morph();
        assert!(m.is_morphing());
        m.tick(2.5); // halfway through 5s
        let t = m.blend_t();
        assert!(
            t > 0.0 && t < 1.0,
            "blend_t should be in (0,1) halfway: {t}"
        );
    }

    #[test]
    fn test_morph_completes() {
        let mut m = make_morph();
        m.start_morph();
        m.tick(10.0); // well past duration
        assert_eq!(m.state(), MorphState::AtB);
        assert_eq!(m.blend_t(), 1.0);
    }

    #[test]
    fn test_morph_reverses() {
        let mut m = make_morph();
        m.start_morph();
        m.tick(10.0); // reach B
        m.start_morph(); // reverse
        m.tick(10.0); // back to A
        assert_eq!(m.state(), MorphState::AtA);
        assert_eq!(m.blend_t(), 0.0);
    }

    #[test]
    fn test_blended_position_at_t0_equals_a() {
        let m = make_morph();
        let pa = m.surface_a.position(0.5, 0.5);
        let pm = m.position(0.5, 0.5);
        let diff = (pa - pm).length();
        assert!(
            diff < 1e-4,
            "at t=0, morphed position should equal surface A: {diff}"
        );
    }

    #[test]
    fn test_blended_mesh_vertex_count() {
        let m = make_morph();
        let (verts, _) = m.blended_mesh(4, 4);
        assert_eq!(verts.len(), 5 * 5);
    }

    #[test]
    fn test_blended_mesh_all_finite() {
        let mut m = make_morph();
        m.start_morph();
        m.tick(2.5);
        let (verts, _) = m.blended_mesh(8, 8);
        for v in &verts {
            assert!(v.iter().all(|x| x.is_finite()), "vertex not finite: {v:?}");
        }
    }

    #[test]
    fn test_smootherstep_endpoints() {
        assert!((smootherstep(0.0) - 0.0).abs() < 1e-6);
        assert!((smootherstep(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_smootherstep_monotone() {
        let a = smootherstep(0.3);
        let b = smootherstep(0.7);
        assert!(a < b, "smootherstep should be monotone increasing");
    }
}
