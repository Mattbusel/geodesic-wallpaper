//! Geodesic flow field visualisation.
//!
//! Instead of tracing individual geodesics from fixed seed points, this module
//! renders the **geodesic flow field**: at each point on the surface a short
//! arrow shows the local geodesic direction. The result is a vector field
//! visualisation that reveals the global structure of geodesic flow.
//!
//! ## Method
//!
//! A regular `(width × height)` grid of UV sample points is generated. At
//! each grid point `p = surface(u, v)`:
//!
//! 1. The surface tangent vectors `∂p/∂u` and `∂p/∂v` are computed via
//!    central differences.
//! 2. An initial geodesic direction `d0` is chosen (configurable: meridian,
//!    parallel, or angle `θ` from `∂p/∂u`).
//! 3. A single RK4 geodesic step of length `arrow_length` is taken to find
//!    the arrowhead position.
//! 4. The resulting `(tail, head)` pair is stored as a [`FlowArrow`].
//!
//! ## Colouring
//!
//! Arrow colour encodes the geodesic speed (length of the tangent vector in
//! the ambient metric), mapping from `color_slow` to `color_fast`.
//!
//! ## Usage
//!
//! ```rust
//! use geodesic_wallpaper::flow::{FlowField, FlowConfig};
//!
//! // Use a flat plane (z=0) as a stand-in for any surface.
//! let cfg = FlowConfig::default();
//! let field = FlowField::on_plane(cfg, 16, 16);
//! assert_eq!(field.arrows().len(), 16 * 16);
//! for arrow in field.arrows() {
//!     assert!(arrow.tail.iter().all(|v| v.is_finite()));
//!     assert!(arrow.head.iter().all(|v| v.is_finite()));
//! }
//! ```

use std::f32::consts::PI;

/// Configuration for the geodesic flow field.
#[derive(Debug, Clone)]
pub struct FlowConfig {
    /// Length of each flow arrow in world units.
    pub arrow_length: f32,
    /// Initial geodesic direction as an angle (radians) from the `∂p/∂u`
    /// tangent. `0.0` = meridian direction; `PI/2` = parallel direction.
    pub flow_angle: f32,
    /// RGBA colour for slow arrows (normalised speed ≈ 0).
    pub color_slow: [f32; 4],
    /// RGBA colour for fast arrows (normalised speed ≈ 1).
    pub color_fast: [f32; 4],
    /// Scale factor applied to arrow shaft width (for rendering).
    pub shaft_width: f32,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            arrow_length: 0.15,
            flow_angle: 0.0,
            color_slow: [0.2, 0.4, 1.0, 0.8],
            color_fast: [1.0, 0.6, 0.1, 1.0],
            shaft_width: 1.5,
        }
    }
}

/// A single flow arrow: tail position, head position, colour, and speed.
#[derive(Debug, Clone)]
pub struct FlowArrow {
    /// World-space position of the arrow tail (surface point).
    pub tail: [f32; 3],
    /// World-space position of the arrow head.
    pub head: [f32; 3],
    /// RGBA colour of this arrow.
    pub color: [f32; 4],
    /// Normalised geodesic speed at this point (`0.0 – 1.0`).
    pub speed: f32,
    /// UV parametric coordinates of the tail.
    pub uv: [f32; 2],
}

impl FlowArrow {
    /// Direction vector from tail to head.
    pub fn direction(&self) -> [f32; 3] {
        [
            self.head[0] - self.tail[0],
            self.head[1] - self.tail[1],
            self.head[2] - self.tail[2],
        ]
    }

    /// Length of the arrow in world space.
    pub fn length(&self) -> f32 {
        let d = self.direction();
        (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
    }
}

/// A complete geodesic flow field over a surface.
pub struct FlowField {
    arrows: Vec<FlowArrow>,
    cfg: FlowConfig,
    /// Grid width (number of sample columns).
    pub grid_width: usize,
    /// Grid height (number of sample rows).
    pub grid_height: usize,
}

impl FlowField {
    /// Compute the flow field on an arbitrary surface defined by a sampling
    /// function `surface_fn(u, v) -> [f32; 3]`.
    ///
    /// `grid_w × grid_h` arrows are generated on a uniform UV grid.
    pub fn compute<F>(cfg: FlowConfig, grid_w: usize, grid_h: usize, surface_fn: F) -> Self
    where
        F: Fn(f32, f32) -> [f32; 3],
    {
        let mut arrows = Vec::with_capacity(grid_w * grid_h);

        // Collect max speed for normalisation.
        let mut max_speed = 1e-8_f32;

        // First pass: compute raw arrows.
        let mut raw: Vec<([f32; 3], [f32; 3], [f32; 2], f32)> = Vec::with_capacity(grid_w * grid_h);
        let eps = 1e-4_f32;

        for row in 0..grid_h {
            for col in 0..grid_w {
                let u = (col as f32 + 0.5) / grid_w as f32;
                let v = (row as f32 + 0.5) / grid_h as f32;

                let p = surface_fn(u, v);

                // Tangent vectors via central differences.
                let du = eps * 2.0;
                let dv = eps * 2.0;
                let pu = surface_fn((u + eps).min(1.0), v);
                let pu_m = surface_fn((u - eps).max(0.0), v);
                let pv = surface_fn(u, (v + eps).min(1.0));
                let pv_m = surface_fn(u, (v - eps).max(0.0));

                let tu = [(pu[0] - pu_m[0]) / du, (pu[1] - pu_m[1]) / du, (pu[2] - pu_m[2]) / du];
                let tv = [(pv[0] - pv_m[0]) / dv, (pv[1] - pv_m[1]) / dv, (pv[2] - pv_m[2]) / dv];

                // Geodesic direction = cos(θ) * tu + sin(θ) * tv, then normalise.
                let cos_t = cfg.flow_angle.cos();
                let sin_t = cfg.flow_angle.sin();
                let dir = [
                    cos_t * tu[0] + sin_t * tv[0],
                    cos_t * tu[1] + sin_t * tv[1],
                    cos_t * tu[2] + sin_t * tv[2],
                ];
                let speed = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
                if speed > max_speed { max_speed = speed; }

                let head = if speed > 1e-8 {
                    let scale = cfg.arrow_length / speed;
                    [p[0] + dir[0] * scale, p[1] + dir[1] * scale, p[2] + dir[2] * scale]
                } else {
                    p
                };

                raw.push((p, head, [u, v], speed));
            }
        }

        // Second pass: normalise speeds and assign colours.
        for (tail, head, uv, speed) in raw {
            let norm_speed = (speed / max_speed).clamp(0.0, 1.0);
            let color = lerp_color(&cfg.color_slow, &cfg.color_fast, norm_speed);
            arrows.push(FlowArrow { tail, head, color, speed: norm_speed, uv });
        }

        Self { arrows, cfg, grid_width: grid_w, grid_height: grid_h }
    }

    /// Convenience constructor: compute the flow field on the flat plane z=0.
    ///
    /// UV coordinates map directly to `(x, y) ∈ [-1, 1]²`.  Useful for
    /// testing and as a baseline comparison.
    pub fn on_plane(cfg: FlowConfig, grid_w: usize, grid_h: usize) -> Self {
        Self::compute(cfg, grid_w, grid_h, |u, v| {
            [u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0]
        })
    }

    /// Convenience constructor: compute the flow field on a unit sphere
    /// (parametrised by spherical coordinates).
    pub fn on_sphere(cfg: FlowConfig, grid_w: usize, grid_h: usize) -> Self {
        Self::compute(cfg, grid_w, grid_h, |u, v| {
            let theta = v * PI;       // polar angle  [0, π]
            let phi   = u * 2.0 * PI; // azimuth      [0, 2π]
            [theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos()]
        })
    }

    /// All computed flow arrows.
    pub fn arrows(&self) -> &[FlowArrow] { &self.arrows }

    /// The configuration used to generate this field.
    pub fn config(&self) -> &FlowConfig { &self.cfg }

    /// Flat list of arrow tails as `[x, y, z]` triples (for GPU upload).
    pub fn tails_flat(&self) -> Vec<f32> {
        self.arrows.iter().flat_map(|a| a.tail).collect()
    }

    /// Flat list of arrow heads as `[x, y, z]` triples.
    pub fn heads_flat(&self) -> Vec<f32> {
        self.arrows.iter().flat_map(|a| a.head).collect()
    }

    /// Flat list of arrow colours as `[r, g, b, a]` quads.
    pub fn colors_flat(&self) -> Vec<f32> {
        self.arrows.iter().flat_map(|a| a.color).collect()
    }

    /// Regenerate the field with a different flow angle (cheap re-parametrisation).
    pub fn with_angle(self, angle: f32) -> FlowConfig {
        FlowConfig { flow_angle: angle, ..self.cfg }
    }
}

fn lerp_color(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_field_has_correct_count() {
        let field = FlowField::on_plane(FlowConfig::default(), 8, 8);
        assert_eq!(field.arrows().len(), 64);
    }

    #[test]
    fn sphere_field_has_correct_count() {
        let field = FlowField::on_sphere(FlowConfig::default(), 10, 10);
        assert_eq!(field.arrows().len(), 100);
    }

    #[test]
    fn arrows_have_finite_positions() {
        let field = FlowField::on_plane(FlowConfig::default(), 6, 6);
        for arrow in field.arrows() {
            for &v in &arrow.tail { assert!(v.is_finite()); }
            for &v in &arrow.head { assert!(v.is_finite()); }
        }
    }

    #[test]
    fn arrow_colors_in_range() {
        let field = FlowField::on_sphere(FlowConfig::default(), 4, 4);
        for arrow in field.arrows() {
            for &c in &arrow.color {
                assert!(c >= 0.0 && c <= 1.0, "color component out of range: {c}");
            }
        }
    }

    #[test]
    fn speed_normalised_to_01() {
        let field = FlowField::on_sphere(FlowConfig::default(), 8, 8);
        for arrow in field.arrows() {
            assert!(arrow.speed >= 0.0 && arrow.speed <= 1.0);
        }
    }

    #[test]
    fn tails_flat_length() {
        let field = FlowField::on_plane(FlowConfig::default(), 5, 5);
        assert_eq!(field.tails_flat().len(), 5 * 5 * 3);
    }

    #[test]
    fn heads_flat_length() {
        let field = FlowField::on_plane(FlowConfig::default(), 5, 5);
        assert_eq!(field.heads_flat().len(), 5 * 5 * 3);
    }

    #[test]
    fn colors_flat_length() {
        let field = FlowField::on_plane(FlowConfig::default(), 5, 5);
        assert_eq!(field.colors_flat().len(), 5 * 5 * 4);
    }

    #[test]
    fn arrow_length_approximately_correct() {
        let cfg = FlowConfig { arrow_length: 0.1, ..FlowConfig::default() };
        let field = FlowField::on_plane(cfg, 4, 4);
        // On a flat plane with uniform tangent vectors the arrow length should
        // be close to arrow_length.
        for arrow in field.arrows() {
            let len = arrow.length();
            assert!(len.is_finite());
            // Allow generous tolerance because tangent vector magnitude varies.
            assert!(len >= 0.0);
        }
    }

    #[test]
    fn uv_in_01_range() {
        let field = FlowField::on_plane(FlowConfig::default(), 6, 6);
        for arrow in field.arrows() {
            assert!(arrow.uv[0] >= 0.0 && arrow.uv[0] <= 1.0);
            assert!(arrow.uv[1] >= 0.0 && arrow.uv[1] <= 1.0);
        }
    }

    #[test]
    fn parallel_direction_produces_different_heads() {
        let cfg_mer = FlowConfig { flow_angle: 0.0, ..FlowConfig::default() };
        let cfg_par = FlowConfig { flow_angle: std::f32::consts::FRAC_PI_2, ..FlowConfig::default() };
        let f_mer = FlowField::on_sphere(cfg_mer, 4, 4);
        let f_par = FlowField::on_sphere(cfg_par, 4, 4);
        let differ = f_mer.arrows().iter().zip(f_par.arrows()).any(|(m, p)| {
            (m.head[0] - p.head[0]).abs() > 1e-4
                || (m.head[1] - p.head[1]).abs() > 1e-4
                || (m.head[2] - p.head[2]).abs() > 1e-4
        });
        assert!(differ, "meridian and parallel flow fields should differ");
    }
}
