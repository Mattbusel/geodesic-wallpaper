//! Lissajous figures, spirograph / hypotrochoid curves, and rose curves.
//!
//! Provides parametric curve generation and a simple rasteriser that draws
//! the curves into an RGB pixel buffer.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// LissajousParams
// ---------------------------------------------------------------------------

/// Parameters for a Lissajous figure.
#[derive(Debug, Clone)]
pub struct LissajousParams {
    /// X-axis frequency multiplier.
    pub a: f64,
    /// Y-axis frequency multiplier.
    pub b: f64,
    /// Phase offset δ (radians).
    pub delta: f64,
    /// X-axis amplitude.
    pub amplitude_x: f64,
    /// Y-axis amplitude.
    pub amplitude_y: f64,
    /// Number of sample points.
    pub num_points: usize,
}

/// Evaluate x = Ax·sin(a·t + δ), y = Ay·sin(b·t).
pub fn lissajous_point(t: f64, params: &LissajousParams) -> (f64, f64) {
    let x = params.amplitude_x * (params.a * t + params.delta).sin();
    let y = params.amplitude_y * (params.b * t).sin();
    (x, y)
}

/// Sample the Lissajous curve at `num_points` equally spaced t ∈ [0, 2π].
pub fn lissajous_curve(params: &LissajousParams) -> Vec<(f64, f64)> {
    let n = params.num_points.max(2);
    (0..n)
        .map(|i| {
            let t = 2.0 * PI * i as f64 / (n - 1) as f64;
            lissajous_point(t, params)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// SpirographParams
// ---------------------------------------------------------------------------

/// Parameters for spirograph (hypo/epi-trochoid) curves.
#[derive(Debug, Clone)]
pub struct SpirographParams {
    /// Radius of the fixed circle.
    pub r_big: f64,
    /// Radius of the rolling circle.
    pub r_small: f64,
    /// Arm length (distance from the rolling circle centre to the pen).
    pub d: f64,
    /// Number of sample points.
    pub num_points: usize,
}

/// Hypotrochoid: x = (R−r)·cos(t) + d·cos((R−r)/r · t)
pub fn spirograph_point(t: f64, p: &SpirographParams) -> (f64, f64) {
    let diff = p.r_big - p.r_small;
    let x = diff * t.cos() + p.d * (diff / p.r_small * t).cos();
    let y = diff * t.sin() - p.d * (diff / p.r_small * t).sin();
    (x, y)
}

/// Epitrochoid: x = (R+r)·cos(t) − d·cos((R+r)/r · t)
pub fn epitrochoid_point(t: f64, p: &SpirographParams) -> (f64, f64) {
    let sum = p.r_big + p.r_small;
    let x = sum * t.cos() - p.d * (sum / p.r_small * t).cos();
    let y = sum * t.sin() - p.d * (sum / p.r_small * t).sin();
    (x, y)
}

// ---------------------------------------------------------------------------
// RoseCurve
// ---------------------------------------------------------------------------

/// Parameters for a rose curve r = a·cos(k·θ).
#[derive(Debug, Clone)]
pub struct RoseCurve {
    /// Petal parameter: odd k → k petals, even k → 2k petals.
    pub k: f64,
    pub amplitude: f64,
    pub num_points: usize,
}

/// Sample a rose curve r = amplitude·cos(k·θ) and convert to Cartesian.
///
/// Uses the full period [0, 2π·lcm_period] to ensure all petals are drawn.
pub fn rose_curve(p: &RoseCurve) -> Vec<(f64, f64)> {
    let n = p.num_points.max(2);
    // For rational k = a/b we'd need a full period; for integer k one full
    // rotation of 2π suffices to draw all petals (for odd k), 4π for even.
    let period = if (p.k as i64) % 2 == 0 {
        4.0 * PI
    } else {
        2.0 * PI
    };
    (0..n)
        .map(|i| {
            let theta = period * i as f64 / (n - 1) as f64;
            let r = p.amplitude * (p.k * theta).cos();
            (r * theta.cos(), r * theta.sin())
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Renderer helpers
// ---------------------------------------------------------------------------

/// Map floating-point curve coordinates to pixel coordinates.
///
/// Returns `None` if the point is outside [0, width) × [0, height).
fn to_pixel(
    x: f64,
    y: f64,
    cx: f64,
    cy: f64,
    scale: f64,
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    let px = (cx + x * scale).round() as i64;
    let py = (cy - y * scale).round() as i64; // flip Y so +y is up
    if px >= 0 && px < width as i64 && py >= 0 && py < height as i64 {
        Some((px as u32, py as u32))
    } else {
        None
    }
}

/// Compute a uniform scale so the curve fits within [margin..width-margin].
fn fit_scale(points: &[(f64, f64)], width: u32, height: u32) -> f64 {
    let margin = 10.0;
    let max_abs_x = points.iter().map(|(x, _)| x.abs()).fold(0.0_f64, f64::max);
    let max_abs_y = points.iter().map(|(_, y)| y.abs()).fold(0.0_f64, f64::max);
    let max_abs = max_abs_x.max(max_abs_y);
    if max_abs < 1e-9 {
        return 1.0;
    }
    let avail_w = (width as f64 / 2.0) - margin;
    let avail_h = (height as f64 / 2.0) - margin;
    avail_w.min(avail_h) / max_abs
}

/// General parametric renderer: draw `points` into an RGB buffer.
///
/// Anti-aliasing is approximated by also lighting the four neighbours of each
/// pixel with a brightness falloff proportional to sub-pixel distance.
pub fn render_parametric(
    points: &[(f64, f64)],
    width: u32,
    height: u32,
    color: [u8; 3],
) -> Vec<u8> {
    let mut buf = vec![0u8; (width * height * 3) as usize];
    if points.is_empty() || width == 0 || height == 0 {
        return buf;
    }

    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let scale = fit_scale(points, width, height);

    let paint = |buf: &mut Vec<u8>, px: i64, py: i64, brightness: f64| {
        if px >= 0 && px < width as i64 && py >= 0 && py < height as i64 {
            let idx = ((py as u32 * width + px as u32) * 3) as usize;
            for (ch, &c) in color.iter().enumerate() {
                let current = buf[idx + ch] as f64;
                let new_val = (current + c as f64 * brightness).min(255.0);
                buf[idx + ch] = new_val as u8;
            }
        }
    };

    for &(x, y) in points {
        let fx = cx + x * scale;
        let fy = cy - y * scale;
        let ix = fx.floor() as i64;
        let iy = fy.floor() as i64;
        let dx = fx - fx.floor();
        let dy = fy - fy.floor();

        // Distribute brightness to surrounding pixels (bilinear-like).
        paint(&mut buf, ix, iy, (1.0 - dx) * (1.0 - dy));
        paint(&mut buf, ix + 1, iy, dx * (1.0 - dy));
        paint(&mut buf, ix, iy + 1, (1.0 - dx) * dy);
        paint(&mut buf, ix + 1, iy + 1, dx * dy);
    }

    buf
}

/// Render a Lissajous figure to an RGB pixel buffer (3 bytes per pixel).
pub fn render_lissajous(params: &LissajousParams, width: u32, height: u32) -> Vec<u8> {
    let points = lissajous_curve(params);
    render_parametric(&points, width, height, [0, 200, 255])
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lissajous_1_1_makes_circle() {
        // a=b=1, delta=π/2 → circle: x=sin(t+π/2)=cos(t), y=sin(t)
        let params = LissajousParams {
            a: 1.0,
            b: 1.0,
            delta: PI / 2.0,
            amplitude_x: 1.0,
            amplitude_y: 1.0,
            num_points: 360,
        };
        let curve = lissajous_curve(&params);
        for (x, y) in &curve {
            let r2 = x * x + y * y;
            assert!((r2 - 1.0).abs() < 1e-6, "r²={} expected 1.0", r2);
        }
    }

    #[test]
    fn rose_k1_has_petals() {
        let p = RoseCurve {
            k: 1.0,
            amplitude: 1.0,
            num_points: 1000,
        };
        let curve = rose_curve(&p);
        // All points should be within the amplitude circle.
        for (x, y) in &curve {
            let r = (x * x + y * y).sqrt();
            assert!(r <= 1.0 + 1e-9, "r={} > amplitude=1.0", r);
        }
    }

    #[test]
    fn spirograph_evaluates_without_nan() {
        let p = SpirographParams {
            r_big: 5.0,
            r_small: 3.0,
            d: 5.0,
            num_points: 1000,
        };
        for i in 0..100 {
            let t = 2.0 * PI * i as f64 / 100.0;
            let (x, y) = spirograph_point(t, &p);
            assert!(x.is_finite() && y.is_finite());
        }
    }

    #[test]
    fn render_returns_correct_buffer_size() {
        let params = LissajousParams {
            a: 3.0,
            b: 2.0,
            delta: PI / 4.0,
            amplitude_x: 1.0,
            amplitude_y: 1.0,
            num_points: 500,
        };
        let buf = render_lissajous(&params, 200, 150);
        assert_eq!(buf.len(), (200 * 150 * 3) as usize);
    }

    #[test]
    fn render_parametric_correct_size() {
        let points = vec![(0.0_f64, 0.0_f64), (0.5, 0.5), (-0.5, -0.5)];
        let buf = render_parametric(&points, 100, 100, [255, 0, 0]);
        assert_eq!(buf.len(), 100 * 100 * 3);
    }
}
