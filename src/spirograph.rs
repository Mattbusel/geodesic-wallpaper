//! Spirograph (hypotrochoid / epitrochoid) curve generator with rasteriser.
//!
//! Generates parametric spirograph curves and renders them to an RGB pixel
//! grid with simple anti-aliasing via circle stamping.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Which variant of the spirograph curve to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpirographType {
    /// Inner wheel rolling inside the outer circle.
    Hypotrochoid,
    /// Inner wheel rolling outside the outer circle.
    Epitrochoid,
}

/// Complete configuration for generating and rendering a spirograph curve.
#[derive(Debug, Clone)]
pub struct SpirographConfig {
    /// Radius of the fixed outer circle.
    pub r: f64,
    /// Radius of the rolling inner circle (rho).
    pub rho: f64,
    /// Distance from the centre of the rolling circle to the tracing point.
    pub d: f64,
    /// Which curve type to generate.
    pub curve_type: SpirographType,
    /// Number of parameter steps (resolution).
    pub steps: usize,
    /// Line colour [R, G, B].
    pub line_color: [u8; 3],
    /// Background colour [R, G, B].
    pub bg_color: [u8; 3],
}

impl Default for SpirographConfig {
    fn default() -> Self {
        Self {
            r: 5.0,
            rho: 3.0,
            d: 5.0,
            curve_type: SpirographType::Hypotrochoid,
            steps: 2000,
            line_color: [255, 200, 0],
            bg_color: [0, 0, 0],
        }
    }
}

/// A 2-D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

// ---------------------------------------------------------------------------
// Parametric curve equations
// ---------------------------------------------------------------------------

/// Hypotrochoid: x = (r-ρ)cos(t) + d·cos((r-ρ)t/ρ)
///               y = (r-ρ)sin(t) − d·sin((r-ρ)t/ρ)
pub fn hypotrochoid(r: f64, rho: f64, d: f64, t: f64) -> Point {
    let diff = r - rho;
    let ratio = if rho != 0.0 { diff / rho } else { 0.0 };
    Point {
        x: diff * t.cos() + d * (ratio * t).cos(),
        y: diff * t.sin() - d * (ratio * t).sin(),
    }
}

/// Epitrochoid: x = (r+ρ)cos(t) − d·cos((r+ρ)t/ρ)
///              y = (r+ρ)sin(t) − d·sin((r+ρ)t/ρ)
pub fn epitrochoid(r: f64, rho: f64, d: f64, t: f64) -> Point {
    let sum = r + rho;
    let ratio = if rho != 0.0 { sum / rho } else { 0.0 };
    Point {
        x: sum * t.cos() - d * (ratio * t).cos(),
        y: sum * t.sin() - d * (ratio * t).sin(),
    }
}

// ---------------------------------------------------------------------------
// Period helpers
// ---------------------------------------------------------------------------

/// Approximate LCM for floating-point values via integer rounding.
pub fn lcm_approx(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 {
        return 0.0;
    }
    // Scale to integers, compute integer LCM, scale back
    let scale = 1_000_000.0;
    let ai = (a * scale).round() as u64;
    let bi = (b * scale).round() as u64;
    let l = lcm_u64(ai, bi);
    l as f64 / scale
}

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn lcm_u64(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        return 0;
    }
    a / gcd_u64(a, b) * b
}

// ---------------------------------------------------------------------------
// Curve generation
// ---------------------------------------------------------------------------

/// Generate the full spirograph curve for the given configuration.
///
/// The parameter t runs from 0 to 2π·lcm(r, ρ)/ρ to produce a closed curve.
pub fn generate_curve(config: &SpirographConfig) -> Vec<Point> {
    let rho = config.rho.abs().max(1e-9);
    let period = 2.0 * PI * lcm_approx(config.r.abs(), rho) / rho;
    // Clamp period to a reasonable value to avoid millions of iterations
    let period = period.min(2.0 * PI * 1_000.0);
    let steps = config.steps.max(1);
    let dt = period / steps as f64;

    (0..=steps)
        .map(|i| {
            let t = i as f64 * dt;
            match config.curve_type {
                SpirographType::Hypotrochoid => hypotrochoid(config.r, rho, config.d, t),
                SpirographType::Epitrochoid => epitrochoid(config.r, rho, config.d, t),
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Rasteriser
// ---------------------------------------------------------------------------

/// Render the spirograph curve to a pixel grid.
///
/// The curve is scaled to fit within the canvas with a small margin.
/// Each curve point is stamped with a circle of radius 1 pixel for simple
/// anti-aliasing.
pub fn render(config: &SpirographConfig, width: u32, height: u32) -> Vec<Vec<[u8; 3]>> {
    let w = width as usize;
    let h = height as usize;

    // Initialise canvas with background colour
    let mut canvas = vec![vec![config.bg_color; w]; h];

    let points = generate_curve(config);
    if points.is_empty() || w == 0 || h == 0 {
        return canvas;
    }

    // Find bounding box
    let min_x = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let max_x = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let min_y = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let max_y = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);

    let range_x = (max_x - min_x).max(1e-9);
    let range_y = (max_y - min_y).max(1e-9);

    let margin = 0.05;
    let inner_w = (w as f64) * (1.0 - 2.0 * margin);
    let inner_h = (h as f64) * (1.0 - 2.0 * margin);

    // Uniform scale preserving aspect ratio
    let scale = (inner_w / range_x).min(inner_h / range_y);
    let offset_x = (w as f64) * margin + (inner_w - range_x * scale) / 2.0;
    let offset_y = (h as f64) * margin + (inner_h - range_y * scale) / 2.0;

    for pt in &points {
        let px = ((pt.x - min_x) * scale + offset_x).round() as i64;
        let py = ((pt.y - min_y) * scale + offset_y).round() as i64;

        // Stamp a 3×3 neighbourhood for anti-aliasing
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let cx = px + dx;
                let cy = py + dy;
                if cx >= 0 && cy >= 0 && cx < w as i64 && cy < h as i64 {
                    canvas[cy as usize][cx as usize] = config.line_color;
                }
            }
        }
    }

    canvas
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hypotrochoid_at_zero() {
        let p = hypotrochoid(5.0, 3.0, 5.0, 0.0);
        // At t=0: x = (5-3) + 5 = 7, y = 0
        assert!((p.x - 7.0).abs() < 1e-9);
        assert!((p.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn epitrochoid_at_zero() {
        let p = epitrochoid(5.0, 3.0, 5.0, 0.0);
        // At t=0: x = (5+3) - 5 = 3, y = 0
        assert!((p.x - 3.0).abs() < 1e-9);
        assert!((p.y - 0.0).abs() < 1e-9);
    }

    #[test]
    fn generate_curve_returns_correct_count() {
        let cfg = SpirographConfig { steps: 100, ..Default::default() };
        let pts = generate_curve(&cfg);
        assert_eq!(pts.len(), 101); // 0..=steps
    }

    #[test]
    fn generate_curve_epitrochoid() {
        let cfg = SpirographConfig {
            curve_type: SpirographType::Epitrochoid,
            steps: 50,
            ..Default::default()
        };
        let pts = generate_curve(&cfg);
        assert!(!pts.is_empty());
    }

    #[test]
    fn lcm_approx_integer_values() {
        let l = lcm_approx(4.0, 6.0);
        assert!((l - 12.0).abs() < 0.01, "lcm(4,6)={l}");
    }

    #[test]
    fn lcm_approx_zero() {
        assert_eq!(lcm_approx(0.0, 5.0), 0.0);
    }

    #[test]
    fn render_size_correct() {
        let cfg = SpirographConfig::default();
        let canvas = render(&cfg, 100, 80);
        assert_eq!(canvas.len(), 80);
        assert!(canvas.iter().all(|row| row.len() == 100));
    }

    #[test]
    fn render_has_non_background_pixels() {
        let cfg = SpirographConfig {
            bg_color: [0, 0, 0],
            line_color: [255, 255, 255],
            steps: 1000,
            ..Default::default()
        };
        let canvas = render(&cfg, 200, 200);
        let non_bg = canvas.iter().flatten().filter(|&&p| p != [0u8, 0, 0]).count();
        assert!(non_bg > 0, "Expected at least one drawn pixel");
    }

    #[test]
    fn render_zero_size_returns_empty() {
        let cfg = SpirographConfig::default();
        let canvas = render(&cfg, 0, 100);
        assert!(canvas.iter().all(|row| row.is_empty()));
    }

    #[test]
    fn curve_is_bounded() {
        let cfg = SpirographConfig::default();
        let pts = generate_curve(&cfg);
        let max_coord = pts.iter().map(|p| p.x.abs().max(p.y.abs())).fold(0.0f64, f64::max);
        // r + rho + d = 5 + 3 + 5 = 13 — the hypotrochoid stays within this
        assert!(max_coord < 20.0, "max_coord={max_coord}");
    }
}
