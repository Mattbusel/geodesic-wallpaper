//! Geodesic field visualization.
//!
//! Instead of rendering a fixed number of individual geodesic trails, this
//! module fills the 2-D parameter domain `(u, v)` with a dense grid of short
//! geodesic segments — one per grid cell — each coloured by the long-term
//! fate of the corresponding geodesic (bounded, escaping, or looping).
//!
//! # Basin classification
//!
//! Each grid cell seeds a geodesic in a random initial direction and
//! integrates it for [`FieldConfig::fate_steps`] RK4 steps.  The final
//! state is compared against three criteria:
//!
//! | Outcome | Condition | Colour role |
//! |---------|-----------|-------------|
//! | `Bounded` | max `(u² + v²)` stays below `escape_radius²` | blue family |
//! | `Escaping` | max `(u² + v²)` exceeds `escape_radius²` | red family |
//! | `Looping` | position returns within `loop_epsilon` of origin | green family |
//!
//! # Rendering
//!
//! [`FieldRenderer`] computes the grid in parallel (using rayon) and emits a
//! flat `Vec<FlowArrow>` that the GPU renderer can upload as an instanced
//! vertex buffer.  The caller is responsible for issuing the draw call.
//!
//! # Usage
//!
//! ```rust,no_run
//! use geodesic_wallpaper::field::{FieldConfig, FieldRenderer};
//!
//! let cfg = FieldConfig::default();
//! let renderer = FieldRenderer::new(cfg);
//! let arrows = renderer.compute(|u, v, du, dv| {
//!     // Your RK4 geodesic step function here — returns (new_u, new_v, new_du, new_dv)
//!     (u + du * 0.01, v + dv * 0.01, du, dv)
//! });
//! println!("Generated {} flow arrows", arrows.len());
//! ```

#![allow(dead_code)]

use rayon::prelude::*;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for the geodesic field renderer.
#[derive(Debug, Clone)]
pub struct FieldConfig {
    /// Number of grid cells along each axis of the `(u, v)` parameter domain.
    ///
    /// Total geodesics computed = `grid_n * grid_n`.
    pub grid_n: usize,
    /// Range of the `u` parameter: `[u_min, u_max]`.
    pub u_range: [f64; 2],
    /// Range of the `v` parameter: `[v_min, v_max]`.
    pub v_range: [f64; 2],
    /// Number of RK4 integration steps used to classify the geodesic fate.
    pub fate_steps: usize,
    /// RK4 integration timestep for fate classification.
    pub fate_dt: f64,
    /// Distance (in parameter space) beyond which a geodesic is classified as escaping.
    pub escape_radius: f64,
    /// Distance (in parameter space) within which a geodesic is classified as looping.
    pub loop_epsilon: f64,
    /// Length of the rendered arrow (in parameter-space units).
    pub arrow_length: f32,
    /// Alpha blending for arrows (0 = invisible, 1 = fully opaque).
    pub arrow_alpha: f32,
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            grid_n: 40,
            u_range: [-std::f64::consts::PI, std::f64::consts::PI],
            v_range: [-std::f64::consts::PI, std::f64::consts::PI],
            fate_steps: 200,
            fate_dt: 0.04,
            escape_radius: 8.0,
            loop_epsilon: 0.3,
            arrow_length: 0.04,
            arrow_alpha: 0.7,
        }
    }
}

// ── Fate classification ────────────────────────────────────────────────────────

/// Long-term fate of a geodesic launched from a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeodesicFate {
    /// The geodesic stays within the bounded region for all `fate_steps`.
    Bounded,
    /// The geodesic leaves the parameter domain (escape radius exceeded).
    Escaping,
    /// The geodesic returns near its starting point (loop detected).
    Looping,
}

impl GeodesicFate {
    /// RGBA colour for this fate class used by the renderer.
    ///
    /// Returns `[r, g, b, a]` in `[0.0, 1.0]` range.
    pub fn color(self, alpha: f32) -> [f32; 4] {
        match self {
            GeodesicFate::Bounded => [0.2, 0.5, 1.0, alpha],
            GeodesicFate::Escaping => [1.0, 0.25, 0.15, alpha],
            GeodesicFate::Looping => [0.2, 0.9, 0.35, alpha],
        }
    }
}

// ── Flow arrow ────────────────────────────────────────────────────────────────

/// A single coloured arrow in the geodesic field.
///
/// Uploaded to the GPU as an instance in an instanced draw call.
/// The arrow starts at `(origin_u, origin_v)` in parameter space and
/// points in direction `(dir_u, dir_v)` (unit vector).
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct FlowArrow {
    /// Arrow base position in parameter space.
    pub origin: [f32; 2],
    /// Normalised direction vector in parameter space.
    pub direction: [f32; 2],
    /// Display length of the arrow in parameter-space units.
    pub length: f32,
    /// RGBA color `[r, g, b, a]`.
    pub color: [f32; 4],
    /// Fate class as an integer for shader-side branching (0=Bounded, 1=Escaping, 2=Looping).
    pub fate: u32,
}

unsafe impl bytemuck::Pod for FlowArrow {}
unsafe impl bytemuck::Zeroable for FlowArrow {}

impl FlowArrow {
    /// Construct from all fields.
    pub fn new(
        origin: [f32; 2],
        direction: [f32; 2],
        length: f32,
        fate: GeodesicFate,
        alpha: f32,
    ) -> Self {
        let fate_int = match fate {
            GeodesicFate::Bounded => 0,
            GeodesicFate::Escaping => 1,
            GeodesicFate::Looping => 2,
        };
        Self {
            origin,
            direction,
            length,
            color: fate.color(alpha),
            fate: fate_int,
        }
    }
}

// ── Geodesic field ────────────────────────────────────────────────────────────

/// Dense grid of geodesic flow arrows covering the parameter domain.
///
/// Stores the last computed set of arrows and metadata about the grid.
pub struct GeodesicField {
    /// Configuration used to compute this field.
    pub config: FieldConfig,
    /// Computed arrows (one per grid cell).
    pub arrows: Vec<FlowArrow>,
    /// Fate classification counts: `[bounded, escaping, looping]`.
    pub fate_counts: [usize; 3],
}

impl GeodesicField {
    /// Returns the fraction of cells classified as each fate.
    pub fn fate_fractions(&self) -> [f32; 3] {
        let total = self.arrows.len().max(1) as f32;
        [
            self.fate_counts[0] as f32 / total,
            self.fate_counts[1] as f32 / total,
            self.fate_counts[2] as f32 / total,
        ]
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

/// Computes geodesic field arrows in parallel using rayon.
///
/// The renderer is stateless; call [`compute`] each time the surface or
/// configuration changes.  For live wallpaper use, recomputing on parameter
/// changes (rather than every frame) is sufficient since the field is a
/// static property of the surface.
pub struct FieldRenderer {
    config: FieldConfig,
}

impl FieldRenderer {
    /// Create a renderer from the given configuration.
    pub fn new(config: FieldConfig) -> Self {
        Self { config }
    }

    /// Compute all flow arrows for the current configuration.
    ///
    /// `step_fn` performs one RK4 geodesic integration step:
    /// given `(u, v, du, dv)` it returns the next `(u, v, du, dv)`.
    ///
    /// The function is called `fate_steps` times per grid cell and must be
    /// thread-safe (it will be called from multiple rayon threads).
    pub fn compute<F>(&self, step_fn: F) -> GeodesicField
    where
        F: Fn(f64, f64, f64, f64) -> (f64, f64, f64, f64) + Send + Sync,
    {
        let n = self.config.grid_n;
        let [u_min, u_max] = self.config.u_range;
        let [v_min, v_max] = self.config.v_range;
        let du_param = (u_max - u_min) / n as f64;
        let dv_param = (v_max - v_min) / n as f64;
        let cfg = &self.config;

        // Build grid cell indices.
        let cells: Vec<(usize, usize)> = (0..n).flat_map(|i| (0..n).map(move |j| (i, j))).collect();

        // Compute arrows in parallel.
        let arrows: Vec<FlowArrow> = cells
            .par_iter()
            .map(|&(i, j)| {
                let u0 = u_min + (i as f64 + 0.5) * du_param;
                let v0 = v_min + (j as f64 + 0.5) * dv_param;

                // Fixed initial direction per cell (deterministic, spread over [0, 2π]).
                let angle = 2.0 * std::f64::consts::PI * (i * n + j) as f64 / (n * n) as f64;
                let du_init = angle.cos();
                let dv_init = angle.sin();

                // Integrate and classify fate.
                let (fate, final_u, final_v) =
                    classify_fate(u0, v0, du_init, dv_init, cfg, &step_fn);

                // Direction arrow points toward where the geodesic went.
                let raw_dir = [(final_u - u0) as f32, (final_v - v0) as f32];
                let dir = normalise2(raw_dir);

                FlowArrow::new(
                    [u0 as f32, v0 as f32],
                    dir,
                    cfg.arrow_length,
                    fate,
                    cfg.arrow_alpha,
                )
            })
            .collect();

        // Count fates.
        let mut fate_counts = [0usize; 3];
        for arrow in &arrows {
            fate_counts[arrow.fate as usize % 3] += 1;
        }

        GeodesicField {
            config: cfg.clone(),
            arrows,
            fate_counts,
        }
    }
}

// ── Fate classification helper ─────────────────────────────────────────────────

/// Integrate a geodesic for `fate_steps` and classify its long-term fate.
///
/// Returns `(fate, final_u, final_v)`.
fn classify_fate<F>(
    u0: f64,
    v0: f64,
    du0: f64,
    dv0: f64,
    cfg: &FieldConfig,
    step_fn: &F,
) -> (GeodesicFate, f64, f64)
where
    F: Fn(f64, f64, f64, f64) -> (f64, f64, f64, f64),
{
    let escape_sq = cfg.escape_radius * cfg.escape_radius;
    let loop_sq = cfg.loop_epsilon * cfg.loop_epsilon;

    let mut u = u0;
    let mut v = v0;
    let mut du = du0;
    let mut dv = dv0;

    for _ in 0..cfg.fate_steps {
        let (nu, nv, ndu, ndv) = step_fn(u, v, du, dv);
        u = nu;
        v = nv;
        du = ndu;
        dv = ndv;

        let dist_sq_from_origin = u * u + v * v;
        if dist_sq_from_origin > escape_sq {
            return (GeodesicFate::Escaping, u, v);
        }

        let return_sq = (u - u0) * (u - u0) + (v - v0) * (v - v0);
        if return_sq < loop_sq {
            return (GeodesicFate::Looping, u, v);
        }
    }

    (GeodesicFate::Bounded, u, v)
}

/// Normalise a 2-D vector; returns `[1, 0]` if the magnitude is near zero.
fn normalise2(v: [f32; 2]) -> [f32; 2] {
    let mag = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if mag < 1e-12 {
        [1.0, 0.0]
    } else {
        [v[0] / mag, v[1] / mag]
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_step(u: f64, v: f64, du: f64, dv: f64) -> (f64, f64, f64, f64) {
        let dt = 0.04;
        (u + du * dt, v + dv * dt, du, dv)
    }

    #[test]
    fn renderer_produces_correct_count() {
        let cfg = FieldConfig {
            grid_n: 5,
            fate_steps: 10,
            ..Default::default()
        };
        let renderer = FieldRenderer::new(cfg);
        let field = renderer.compute(identity_step);
        assert_eq!(field.arrows.len(), 25, "5×5 grid should yield 25 arrows");
    }

    #[test]
    fn fate_fractions_sum_to_one() {
        let cfg = FieldConfig {
            grid_n: 4,
            fate_steps: 10,
            ..Default::default()
        };
        let renderer = FieldRenderer::new(cfg);
        let field = renderer.compute(identity_step);
        let fracs = field.fate_fractions();
        let sum: f32 = fracs.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-5,
            "fate fractions should sum to 1, got {sum}"
        );
    }

    #[test]
    fn escape_detected_for_fast_step() {
        // A step function that immediately balloons the position.
        let fast_step =
            |_u: f64, _v: f64, _du: f64, _dv: f64| (1000.0_f64, 1000.0_f64, 0.0_f64, 0.0_f64);
        let cfg = FieldConfig {
            grid_n: 2,
            fate_steps: 5,
            escape_radius: 10.0,
            ..Default::default()
        };
        let renderer = FieldRenderer::new(cfg);
        let field = renderer.compute(fast_step);
        // All cells should be classified as Escaping.
        assert!(
            field.fate_counts[1] == 4,
            "all 4 cells should escape, got {:?}",
            field.fate_counts
        );
    }

    #[test]
    fn normalise2_unit_vector() {
        let n = normalise2([3.0, 4.0]);
        let mag = (n[0] * n[0] + n[1] * n[1]).sqrt();
        assert!((mag - 1.0).abs() < 1e-6, "normalised magnitude should be 1");
    }

    #[test]
    fn normalise2_zero_vector_fallback() {
        let n = normalise2([0.0, 0.0]);
        assert_eq!(n, [1.0, 0.0]);
    }

    #[test]
    fn flow_arrow_fate_integer_mapping() {
        let a = FlowArrow::new([0.0, 0.0], [1.0, 0.0], 1.0, GeodesicFate::Bounded, 1.0);
        assert_eq!(a.fate, 0);
        let b = FlowArrow::new([0.0, 0.0], [1.0, 0.0], 1.0, GeodesicFate::Escaping, 1.0);
        assert_eq!(b.fate, 1);
        let c = FlowArrow::new([0.0, 0.0], [1.0, 0.0], 1.0, GeodesicFate::Looping, 1.0);
        assert_eq!(c.fate, 2);
    }

    #[test]
    fn geodesic_fate_colors_in_range() {
        for fate in [
            GeodesicFate::Bounded,
            GeodesicFate::Escaping,
            GeodesicFate::Looping,
        ] {
            let c = fate.color(0.8);
            for ch in c.iter() {
                assert!(*ch >= 0.0 && *ch <= 1.0, "color channel {ch} out of range");
            }
        }
    }
}
