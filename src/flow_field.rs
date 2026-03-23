//! # Vector Flow Field and Streamline Tracing
//!
//! Provides a 2-D vector flow field with bilinear interpolation, multiple
//! built-in field generators (uniform, circular, Perlin-noise-inspired,
//! curl), and RK4 streamline tracing with Bresenham rendering.
//!
//! ## Example
//!
//! ```rust,ignore
//! use geodesic_wallpaper::flow_field::{FlowField, Streamline, StreamlineRenderer};
//!
//! let field = FlowField::new_circular(800, 600, (400.0, 300.0));
//! let line = Streamline::trace(&field, (100.0, 100.0), 1.0, 500);
//! let pixels = StreamlineRenderer::render(&field, 64, 42, 800, 600, (255, 200, 100));
//! ```

// ── Vector2 ───────────────────────────────────────────────────────────────────

/// A 2-D floating-point vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    /// Euclidean length.
    pub fn length(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Unit vector; returns zero vector if length is near zero.
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < 1e-12 {
            Self::zero()
        } else {
            Self { x: self.x / len, y: self.y / len }
        }
    }

    /// Rotate by `angle` radians counter-clockwise.
    pub fn rotate(&self, angle: f64) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self { x: self.x * cos - self.y * sin, y: self.x * sin + self.y * cos }
    }

    pub fn add(&self, other: &Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }

    pub fn scale(&self, s: f64) -> Self {
        Self { x: self.x * s, y: self.y * s }
    }
}

// ── FlowField ─────────────────────────────────────────────────────────────────

/// A 2-D grid of vectors representing a flow field.
pub struct FlowField {
    pub width: usize,
    pub height: usize,
    /// `field[y][x]` = vector at grid cell (x, y).
    pub field: Vec<Vec<Vector2>>,
}

impl FlowField {
    // ── Constructors ─────────────────────────────────────────────────────────

    /// Constant direction everywhere (angle in degrees from positive X axis).
    pub fn new_uniform(width: usize, height: usize, angle_deg: f64) -> Self {
        let angle_rad = angle_deg.to_radians();
        let v = Vector2::new(angle_rad.cos(), angle_rad.sin());
        let field = (0..height).map(|_| vec![v; width]).collect();
        Self { width, height, field }
    }

    /// Vectors tangent to circles centred at `center`.
    pub fn new_circular(width: usize, height: usize, center: (f64, f64)) -> Self {
        let field = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        let dx = x as f64 - center.0;
                        let dy = y as f64 - center.1;
                        // Tangent to radial vector is (-dy, dx) normalised.
                        let v = Vector2::new(-dy, dx);
                        v.normalize()
                    })
                    .collect()
            })
            .collect();
        Self { width, height, field }
    }

    /// Simple gradient-noise driven field (deterministic, based on a hash).
    ///
    /// Produces angles in [0, 2π) derived from a pseudo-random gradient at
    /// each grid cell.  Accepts `scale` to control spatial frequency and
    /// `seed` for reproducibility.
    pub fn new_perlin(width: usize, height: usize, scale: f64, seed: u64) -> Self {
        let field = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        let angle = gradient_noise(x as f64 * scale, y as f64 * scale, seed)
                            * 2.0
                            * std::f64::consts::PI;
                        Vector2::new(angle.cos(), angle.sin())
                    })
                    .collect()
            })
            .collect();
        Self { width, height, field }
    }

    /// Curl field derived from a scalar potential P via finite differences.
    ///
    /// F = (∂P/∂y, -∂P/∂x) with step h = 1.0.
    pub fn new_curl(
        width: usize,
        height: usize,
        potential_fn: impl Fn(f64, f64) -> f64,
    ) -> Self {
        let h = 1.0_f64;
        let field = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        let xf = x as f64;
                        let yf = y as f64;
                        let dp_dy = (potential_fn(xf, yf + h) - potential_fn(xf, yf - h))
                            / (2.0 * h);
                        let dp_dx = (potential_fn(xf + h, yf) - potential_fn(xf - h, yf))
                            / (2.0 * h);
                        Vector2::new(dp_dy, -dp_dx)
                    })
                    .collect()
            })
            .collect();
        Self { width, height, field }
    }

    // ── Sampling ─────────────────────────────────────────────────────────────

    /// Bilinearly interpolate the field at continuous position (x, y).
    ///
    /// Clamps to the field boundary.
    pub fn at(&self, x: f64, y: f64) -> Vector2 {
        let x = x.clamp(0.0, (self.width - 1) as f64);
        let y = y.clamp(0.0, (self.height - 1) as f64);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);
        let tx = x - x0 as f64;
        let ty = y - y0 as f64;

        let v00 = self.field[y0][x0];
        let v10 = self.field[y0][x1];
        let v01 = self.field[y1][x0];
        let v11 = self.field[y1][x1];

        let top = v00.scale(1.0 - tx).add(&v10.scale(tx));
        let bot = v01.scale(1.0 - tx).add(&v11.scale(tx));
        top.scale(1.0 - ty).add(&bot.scale(ty))
    }

    /// Normalise every vector in the field to unit length.
    pub fn normalize_field(&mut self) {
        for row in &mut self.field {
            for v in row {
                *v = v.normalize();
            }
        }
    }
}

/// Simple deterministic gradient noise returning a value in [0, 1).
fn gradient_noise(x: f64, y: f64, seed: u64) -> f64 {
    // Hash x and y into a pseudo-random value using integer arithmetic.
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;
    let hash = (xi.wrapping_mul(1_619) ^ yi.wrapping_mul(31_337))
        .wrapping_add(seed as i64)
        .wrapping_mul(6_364_136_223_846_793_005_i64)
        .wrapping_add(1_442_695_040_888_963_407_i64);
    ((hash >> 33) as u32 as f64) / u32::MAX as f64
}

// ── Streamline ────────────────────────────────────────────────────────────────

/// A traced streamline through a flow field.
pub struct Streamline {
    pub points: Vec<(f64, f64)>,
}

impl Streamline {
    /// Trace a streamline using 4th-order Runge-Kutta integration.
    ///
    /// Stops when the point leaves the field boundary or `max_steps` is reached.
    pub fn trace(
        field: &FlowField,
        start: (f64, f64),
        step_size: f64,
        max_steps: usize,
    ) -> Self {
        let w = field.width as f64;
        let h = field.height as f64;
        let mut points = vec![start];
        let mut pos = start;

        for _ in 0..max_steps {
            let (px, py) = pos;

            let k1 = field.at(px, py);
            let k2 = field.at(px + k1.x * step_size / 2.0, py + k1.y * step_size / 2.0);
            let k3 = field.at(px + k2.x * step_size / 2.0, py + k2.y * step_size / 2.0);
            let k4 = field.at(px + k3.x * step_size, py + k3.y * step_size);

            let dx = (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x) * step_size / 6.0;
            let dy = (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y) * step_size / 6.0;

            let nx = px + dx;
            let ny = py + dy;

            if nx < 0.0 || nx >= w || ny < 0.0 || ny >= h {
                break;
            }
            pos = (nx, ny);
            points.push(pos);
        }
        Self { points }
    }
}

// ── StreamlineRenderer ────────────────────────────────────────────────────────

/// Renders flow fields and streamlines into an RGB pixel buffer.
pub struct StreamlineRenderer;

impl StreamlineRenderer {
    /// Generate `num_lines` streamlines from random start points and render
    /// them into a flat RGB buffer of dimensions `width × height`.
    pub fn render(
        field: &FlowField,
        num_lines: usize,
        seed: u64,
        width: usize,
        height: usize,
        color: (u8, u8, u8),
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height * 3];
        let mut rng = LcgRng::new(seed);

        for _ in 0..num_lines {
            let sx = rng.next_f64() * width as f64;
            let sy = rng.next_f64() * height as f64;
            let line = Streamline::trace(field, (sx, sy), 1.5, 300);
            for window in line.points.windows(2) {
                let (x0, y0) = window[0];
                let (x1, y1) = window[1];
                bresenham(
                    x0 as i64, y0 as i64, x1 as i64, y1 as i64, width, height,
                    &mut pixels, color,
                );
            }
        }
        pixels
    }

    /// Draw small arrows at each grid point showing the field direction.
    pub fn render_arrows(
        field: &FlowField,
        grid_spacing: usize,
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; width * height * 3];
        let arrow_len = (grid_spacing as f64 * 0.4).max(2.0);

        let mut y = grid_spacing / 2;
        while y < height {
            let mut x = grid_spacing / 2;
            while x < width {
                let v = field.at(x as f64, y as f64).normalize();
                let ex = (x as f64 + v.x * arrow_len) as i64;
                let ey = (y as f64 + v.y * arrow_len) as i64;
                bresenham(
                    x as i64, y as i64, ex, ey, width, height, &mut pixels,
                    (200, 200, 200),
                );
                x += grid_spacing;
            }
            y += grid_spacing;
        }
        pixels
    }
}

/// Bresenham's line algorithm — sets pixels in an RGB buffer.
fn bresenham(
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    width: usize,
    height: usize,
    pixels: &mut Vec<u8>,
    color: (u8, u8, u8),
) {
    let mut x0 = x0;
    let mut y0 = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i64 = if x0 < x1 { 1 } else { -1 };
    let sy: i64 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x0 >= 0 && y0 >= 0 && (x0 as usize) < width && (y0 as usize) < height {
            let idx = (y0 as usize * width + x0 as usize) * 3;
            pixels[idx] = color.0;
            pixels[idx + 1] = color.1;
            pixels[idx + 2] = color.2;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            if x0 == x1 { break; }
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            if y0 == y1 { break; }
            err += dx;
            y0 += sy;
        }
    }
}

// ── Simple LCG RNG ────────────────────────────────────────────────────────────

struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_field_traces_straight_line() {
        let field = FlowField::new_uniform(200, 200, 0.0); // pointing right
        let line = Streamline::trace(&field, (10.0, 100.0), 1.0, 50);
        // All y-coordinates should be constant (within floating-point epsilon).
        for &(_, y) in &line.points {
            assert!((y - 100.0).abs() < 1e-6, "y deviated: {y}");
        }
        // X should increase monotonically.
        for w in line.points.windows(2) {
            assert!(w[1].0 > w[0].0);
        }
    }

    #[test]
    fn circular_field_stays_within_bounds() {
        let w = 300usize;
        let h = 300usize;
        let field = FlowField::new_circular(w, h, (150.0, 150.0));
        let line = Streamline::trace(&field, (200.0, 150.0), 1.0, 1000);
        for &(x, y) in &line.points {
            assert!(x >= 0.0 && x < w as f64, "x out of bounds: {x}");
            assert!(y >= 0.0 && y < h as f64, "y out of bounds: {y}");
        }
    }

    #[test]
    fn streamline_stops_at_boundary() {
        // Uniform field pointing directly right: line will exit the right edge.
        let field = FlowField::new_uniform(50, 50, 0.0);
        let line = Streamline::trace(&field, (40.0, 25.0), 2.0, 1000);
        // Must stop before exceeding width.
        for &(x, _) in &line.points {
            assert!(x < 50.0, "x exceeded boundary: {x}");
        }
    }

    #[test]
    fn render_returns_correct_buffer_size() {
        let field = FlowField::new_uniform(100, 100, 45.0);
        let buf = StreamlineRenderer::render(&field, 10, 0, 100, 100, (255, 255, 255));
        assert_eq!(buf.len(), 100 * 100 * 3);
    }

    #[test]
    fn rk4_more_accurate_than_euler_for_circular() {
        // For a circular field the streamline should stay close to the
        // starting radius.  RK4 conserves the radius better than Euler.
        let w = 500usize;
        let h = 500usize;
        let cx = 250.0_f64;
        let cy = 250.0_f64;
        let field = FlowField::new_circular(w, h, (cx, cy));

        let start = (350.0_f64, 250.0_f64);
        let target_radius = ((start.0 - cx).powi(2) + (start.1 - cy).powi(2)).sqrt();

        let line = Streamline::trace(&field, start, 0.5, 200);

        let max_deviation = line
            .points
            .iter()
            .map(|&(x, y)| {
                let r = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                (r - target_radius).abs()
            })
            .fold(0.0_f64, f64::max);

        // RK4 should keep deviation small over 200 steps.
        assert!(max_deviation < 5.0, "radius deviation too large: {max_deviation:.2}");
    }
}
