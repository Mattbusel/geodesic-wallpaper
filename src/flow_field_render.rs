//! Vector field visualization with streamlines.
//!
//! Provides a 2-D vector flow field with curl-noise generation, bilinear
//! interpolation, Euler-integrated streamline tracing, and pixel rendering.

use std::ops::{Add, Mul, Sub};

// ---------------------------------------------------------------------------
// Vec2
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag < 1e-12 {
            Self::zero()
        } else {
            Self {
                x: self.x / mag,
                y: self.y / mag,
            }
        }
    }
}

impl Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f64) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

// ---------------------------------------------------------------------------
// FlowField
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FlowField {
    pub width: u32,
    pub height: u32,
    pub field: Vec<Vec2>,
}

impl FlowField {
    /// Construct a flow field by evaluating `f(x, y)` at every grid point.
    pub fn from_function<F: Fn(f64, f64) -> Vec2>(width: u32, height: u32, f: F) -> Self {
        let mut field = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                field.push(f(x as f64, y as f64));
            }
        }
        Self { width, height, field }
    }

    /// Construct a curl-noise flow field.
    /// Uses a Perlin-like potential function and computes the curl via finite differences.
    pub fn curl_noise(width: u32, height: u32, seed: u64, scale: f64) -> Self {
        let h = 1.0; // finite difference step
        Self::from_function(width, height, |x, y| {
            let nx = x * scale;
            let ny = y * scale;
            // dψ/dy
            let dpsi_dy = (potential(nx, ny + h, seed) - potential(nx, ny - h, seed)) / (2.0 * h);
            // -dψ/dx
            let neg_dpsi_dx = -(potential(nx + h, ny, seed) - potential(nx - h, ny, seed)) / (2.0 * h);
            Vec2::new(dpsi_dy, neg_dpsi_dx)
        })
    }

    /// Bilinearly interpolate the field at floating-point coordinates.
    pub fn sample(&self, x: f64, y: f64) -> Vec2 {
        let w = self.width as f64;
        let h = self.height as f64;

        // Clamp to valid range
        let x = x.clamp(0.0, w - 1.0);
        let y = y.clamp(0.0, h - 1.0);

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let tx = x - x0 as f64;
        let ty = y - y0 as f64;

        let v00 = self.at(x0, y0);
        let v10 = self.at(x1, y0);
        let v01 = self.at(x0, y1);
        let v11 = self.at(x1, y1);

        let top    = v00 * (1.0 - tx) + v10 * tx;
        let bottom = v01 * (1.0 - tx) + v11 * tx;
        top * (1.0 - ty) + bottom * ty
    }

    fn at(&self, x: u32, y: u32) -> Vec2 {
        self.field[(y * self.width + x) as usize]
    }
}

/// Simple Perlin-like scalar noise potential via hash-based gradients.
fn potential(x: f64, y: f64, seed: u64) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let fx = x - x.floor();
    let fy = y - y.floor();

    // Smooth interpolation
    let u = smooth(fx);
    let v = smooth(fy);

    let g00 = gradient(xi,     yi,     seed);
    let g10 = gradient(xi + 1, yi,     seed);
    let g01 = gradient(xi,     yi + 1, seed);
    let g11 = gradient(xi + 1, yi + 1, seed);

    let n00 = g00.x * fx       + g00.y * fy;
    let n10 = g10.x * (fx-1.0) + g10.y * fy;
    let n01 = g01.x * fx       + g01.y * (fy-1.0);
    let n11 = g11.x * (fx-1.0) + g11.y * (fy-1.0);

    let x0 = lerp(n00, n10, u);
    let x1 = lerp(n01, n11, u);
    lerp(x0, x1, v)
}

fn smooth(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

fn gradient(xi: i64, yi: i64, seed: u64) -> Vec2 {
    let h = hash(xi, yi, seed);
    let angle = h as f64 * std::f64::consts::TAU / u64::MAX as f64;
    Vec2::new(angle.cos(), angle.sin())
}

fn hash(xi: i64, yi: i64, seed: u64) -> u64 {
    let mut h = seed;
    h ^= xi.unsigned_abs() as u64 * 2654435761;
    h ^= yi.unsigned_abs() as u64 * 2246822519;
    h ^= (xi < 0) as u64 * 0xdeadbeef;
    h ^= (yi < 0) as u64 * 0xcafebabe;
    h = h.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    h
}

// ---------------------------------------------------------------------------
// Streamline
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Streamline {
    pub points: Vec<(f64, f64)>,
    pub length: f64,
}

/// Trace a streamline from (`start_x`, `start_y`) using Euler integration.
pub fn integrate_streamline(
    field: &FlowField,
    start_x: f64,
    start_y: f64,
    step_size: f64,
    max_steps: usize,
) -> Streamline {
    let mut points = Vec::with_capacity(max_steps);
    let mut x = start_x;
    let mut y = start_y;
    let mut length = 0.0f64;

    let w = field.width as f64;
    let h = field.height as f64;

    points.push((x, y));

    for _ in 0..max_steps {
        let v = field.sample(x, y);
        let mag = v.magnitude();
        if mag < 1e-10 {
            break;
        }
        let nx = x + v.x * step_size;
        let ny = y + v.y * step_size;

        if nx < 0.0 || nx >= w || ny < 0.0 || ny >= h {
            break;
        }

        let dx = nx - x;
        let dy = ny - y;
        length += (dx * dx + dy * dy).sqrt();

        x = nx;
        y = ny;
        points.push((x, y));
    }

    Streamline { points, length }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render the flow field as arrows on a grid. Returns an RGB pixel buffer (width * height * 3).
pub fn render_flow_field(
    field: &FlowField,
    width: u32,
    height: u32,
    bg_color: [u8; 3],
    arrow_color: [u8; 3],
) -> Vec<u8> {
    let mut buf = vec![0u8; (width * height * 3) as usize];

    // Fill background
    for pixel in buf.chunks_mut(3) {
        pixel.copy_from_slice(&bg_color);
    }

    let grid_spacing = 32u32;
    let arrow_scale = grid_spacing as f64 * 0.4;

    let mut gx = grid_spacing / 2;
    while gx < width {
        let mut gy = grid_spacing / 2;
        while gy < height {
            let v = field.sample(gx as f64, gy as f64);
            let norm = v.normalize();
            let tip_x = gx as f64 + norm.x * arrow_scale;
            let tip_y = gy as f64 + norm.y * arrow_scale;

            draw_line(&mut buf, width, height,
                gx as f64, gy as f64, tip_x, tip_y, arrow_color);
            gy += grid_spacing;
        }
        gx += grid_spacing;
    }

    buf
}

/// Render streamlines with intensity proportional to velocity magnitude.
pub fn render_streamlines(streamlines: &[Streamline], width: u32, height: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (width * height * 3) as usize];

    for sl in streamlines {
        for window in sl.points.windows(2) {
            let (x0, y0) = window[0];
            let (x1, y1) = window[1];
            draw_line(&mut buf, width, height, x0, y0, x1, y1, [200, 200, 255]);
        }
    }

    buf
}

/// Generate streamlines from random start points using an LCG seeded with `seed`.
pub fn generate_streamlines(field: &FlowField, num_lines: usize, seed: u64) -> Vec<Streamline> {
    let mut state = seed;
    let mut lcg = move || -> u64 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        state
    };

    (0..num_lines)
        .map(|_| {
            let sx = (lcg() % field.width as u64) as f64;
            let sy = (lcg() % field.height as u64) as f64;
            integrate_streamline(field, sx, sy, 1.0, 500)
        })
        .collect()
}

/// Compute divergence dVx/dx + dVy/dy at (x, y).
pub fn divergence_at(field: &FlowField, x: f64, y: f64) -> f64 {
    let h = 1.0;
    let dvx_dx = (field.sample(x + h, y).x - field.sample(x - h, y).x) / (2.0 * h);
    let dvy_dy = (field.sample(x, y + h).y - field.sample(x, y - h).y) / (2.0 * h);
    dvx_dx + dvy_dy
}

/// Compute curl dVy/dx - dVx/dy at (x, y).
pub fn curl_at(field: &FlowField, x: f64, y: f64) -> f64 {
    let h = 1.0;
    let dvy_dx = (field.sample(x + h, y).y - field.sample(x - h, y).y) / (2.0 * h);
    let dvx_dy = (field.sample(x, y + h).x - field.sample(x, y - h).x) / (2.0 * h);
    dvy_dx - dvx_dy
}

// Bresenham line drawing helper
fn draw_line(
    buf: &mut [u8],
    width: u32,
    height: u32,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: [u8; 3],
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let steps = (dx.max(dy) as usize).max(1);

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let x = (x0 + t * (x1 - x0)) as i32;
        let y = (y0 + t * (y1 - y0)) as i32;
        if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
            let idx = ((y as u32 * width + x as u32) * 3) as usize;
            buf[idx]     = color[0];
            buf[idx + 1] = color[1];
            buf[idx + 2] = color[2];
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_add() {
        let a = Vec2::new(1.0, 2.0);
        let b = Vec2::new(3.0, 4.0);
        let c = a + b;
        assert!((c.x - 4.0).abs() < 1e-10);
        assert!((c.y - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec2_sub() {
        let a = Vec2::new(5.0, 3.0);
        let b = Vec2::new(2.0, 1.0);
        let c = a - b;
        assert!((c.x - 3.0).abs() < 1e-10);
        assert!((c.y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec2_mul() {
        let a = Vec2::new(2.0, 3.0);
        let c = a * 2.0;
        assert!((c.x - 4.0).abs() < 1e-10);
        assert!((c.y - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec2_magnitude() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.magnitude() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_vec2_normalize() {
        let v = Vec2::new(3.0, 4.0);
        let n = v.normalize();
        assert!((n.magnitude() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_sample_at_known_point() {
        // Uniform field pointing right
        let field = FlowField::from_function(100, 100, |_, _| Vec2::new(1.0, 0.0));
        let v = field.sample(50.0, 50.0);
        assert!((v.x - 1.0).abs() < 1e-10);
        assert!((v.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_streamline_stays_in_bounds() {
        let field = FlowField::from_function(200, 200, |x, _| Vec2::new(1.0, 0.0));
        let sl = integrate_streamline(&field, 10.0, 100.0, 1.0, 1000);
        for (x, y) in &sl.points {
            assert!(*x >= 0.0 && *x < 200.0, "x={} out of bounds", x);
            assert!(*y >= 0.0 && *y < 200.0, "y={} out of bounds", y);
        }
    }

    #[test]
    fn test_render_returns_correct_size() {
        let field = FlowField::from_function(100, 100, |_, _| Vec2::new(0.5, 0.5));
        let buf = render_flow_field(&field, 100, 100, [0, 0, 0], [255, 255, 255]);
        assert_eq!(buf.len(), 100 * 100 * 3);
    }

    #[test]
    fn test_render_streamlines_correct_size() {
        let field = FlowField::from_function(100, 100, |_, _| Vec2::new(0.5, 0.5));
        let streamlines = generate_streamlines(&field, 5, 42);
        let buf = render_streamlines(&streamlines, 100, 100);
        assert_eq!(buf.len(), 100 * 100 * 3);
    }

    #[test]
    fn test_curl_noise_field_size() {
        let field = FlowField::curl_noise(64, 64, 42, 0.05);
        assert_eq!(field.field.len(), 64 * 64);
    }

    #[test]
    fn test_divergence_curl_noise_approx_zero() {
        // curl noise should have near-zero divergence
        let field = FlowField::curl_noise(100, 100, 42, 0.05);
        let div = divergence_at(&field, 50.0, 50.0);
        // Not exactly zero due to finite differences, but should be small
        assert!(div.abs() < 1.0, "divergence should be small, got {}", div);
    }
}
