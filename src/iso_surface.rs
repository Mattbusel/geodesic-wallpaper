//! Marching squares iso-surface extraction for 2-D scalar fields.
//!
//! Provides [`ScalarField2D`] for storing or sampling scalar data, and
//! [`MarchingSquares`] for extracting contour lines at arbitrary iso-levels.

// ---------------------------------------------------------------------------
// ScalarField2D
// ---------------------------------------------------------------------------

/// A 2-D grid of scalar values with optional bilinear sampling.
pub struct ScalarField2D {
    pub values: Vec<Vec<f64>>,
    pub width: usize,
    pub height: usize,
    pub x_range: (f64, f64),
    pub y_range: (f64, f64),
}

impl ScalarField2D {
    /// Create a zero-filled field.
    pub fn new(
        width: usize,
        height: usize,
        x_range: (f64, f64),
        y_range: (f64, f64),
    ) -> Self {
        ScalarField2D {
            values: vec![vec![0.0; width]; height],
            width,
            height,
            x_range,
            y_range,
        }
    }

    /// Set a grid value (row j, column i).
    pub fn set(&mut self, i: usize, j: usize, val: f64) {
        if j < self.height && i < self.width {
            self.values[j][i] = val;
        }
    }

    /// Get a grid value.
    pub fn get(&self, i: usize, j: usize) -> f64 {
        if j < self.height && i < self.width {
            self.values[j][i]
        } else {
            0.0
        }
    }

    /// Build a field by evaluating `f(x, y)` over the grid.
    pub fn from_function<F>(
        width: usize,
        height: usize,
        x_range: (f64, f64),
        y_range: (f64, f64),
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        let mut field = Self::new(width, height, x_range, y_range);
        for j in 0..height {
            for i in 0..width {
                let x = x_range.0 + (x_range.1 - x_range.0) * i as f64 / (width - 1).max(1) as f64;
                let y = y_range.0 + (y_range.1 - y_range.0) * j as f64 / (height - 1).max(1) as f64;
                field.values[j][i] = f(x, y);
            }
        }
        field
    }

    /// Bilinear sample at world coordinate `(x, y)`.
    pub fn sample_bilinear(&self, x: f64, y: f64) -> f64 {
        if self.width < 2 || self.height < 2 {
            return 0.0;
        }
        let tx = (x - self.x_range.0) / (self.x_range.1 - self.x_range.0);
        let ty = (y - self.y_range.0) / (self.y_range.1 - self.y_range.0);
        let tx = tx.clamp(0.0, 1.0);
        let ty = ty.clamp(0.0, 1.0);

        let fx = tx * (self.width - 1) as f64;
        let fy = ty * (self.height - 1) as f64;

        let i0 = (fx as usize).min(self.width - 2);
        let j0 = (fy as usize).min(self.height - 2);
        let sf = fx - i0 as f64;
        let tf = fy - j0 as f64;

        let v00 = self.values[j0][i0];
        let v10 = self.values[j0][i0 + 1];
        let v01 = self.values[j0 + 1][i0];
        let v11 = self.values[j0 + 1][i0 + 1];

        v00 * (1.0 - sf) * (1.0 - tf)
            + v10 * sf * (1.0 - tf)
            + v01 * (1.0 - sf) * tf
            + v11 * sf * tf
    }

    /// World-space x coordinate for grid column `i`.
    fn world_x(&self, i: usize) -> f64 {
        self.x_range.0
            + (self.x_range.1 - self.x_range.0) * i as f64 / (self.width - 1).max(1) as f64
    }

    /// World-space y coordinate for grid row `j`.
    fn world_y(&self, j: usize) -> f64 {
        self.y_range.0
            + (self.y_range.1 - self.y_range.0) * j as f64 / (self.height - 1).max(1) as f64
    }
}

// ---------------------------------------------------------------------------
// LineSegment / ContourLevel
// ---------------------------------------------------------------------------

/// An oriented line segment produced by marching squares.
#[derive(Debug, Clone)]
pub struct LineSegment {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// A collection of segments at a specific iso-level with an associated colour.
#[derive(Debug, Clone)]
pub struct ContourLevel {
    pub level: f64,
    pub segments: Vec<LineSegment>,
    pub color: [u8; 3],
}

// ---------------------------------------------------------------------------
// MarchingSquares
// ---------------------------------------------------------------------------

/// Marching-squares contour extractor.
pub struct MarchingSquares;

impl MarchingSquares {
    /// Linear interpolation: where on the edge `[v0, v1]` does the contour at
    /// `level` cross?  Returns a value in [0, 1].
    pub fn interpolate_edge(v0: f64, v1: f64, level: f64) -> f64 {
        if (v1 - v0).abs() < 1e-12 {
            0.5
        } else {
            ((level - v0) / (v1 - v0)).clamp(0.0, 1.0)
        }
    }

    /// Extract all contour segments for a single iso-level.
    ///
    /// Implements all 16 marching-squares cases (including the two ambiguous
    /// cases 5 and 10 which are resolved by averaging).
    pub fn extract(field: &ScalarField2D, level: f64) -> Vec<LineSegment> {
        let mut segments = Vec::new();
        if field.width < 2 || field.height < 2 {
            return segments;
        }

        for j in 0..field.height - 1 {
            for i in 0..field.width - 1 {
                // Cell corners (counter-clockwise from bottom-left)
                // 0 = bottom-left, 1 = bottom-right, 2 = top-right, 3 = top-left
                let v0 = field.get(i,     j + 1); // bottom-left  (x0, y1)
                let v1 = field.get(i + 1, j + 1); // bottom-right (x1, y1)
                let v2 = field.get(i + 1, j);     // top-right    (x1, y0)
                let v3 = field.get(i,     j);     // top-left     (x0, y0)

                let x0 = field.world_x(i);
                let x1 = field.world_x(i + 1);
                let y0 = field.world_y(j);     // top y (smaller j)
                let y1 = field.world_y(j + 1); // bottom y

                // Build 4-bit index: bit k = 1 if v_k >= level
                let b0 = (v0 >= level) as u8;
                let b1 = (v1 >= level) as u8;
                let b2 = (v2 >= level) as u8;
                let b3 = (v3 >= level) as u8;
                let case = b0 | (b1 << 1) | (b2 << 2) | (b3 << 3);

                // Edge midpoints (linear interpolation)
                // Edge 0: bottom (v0 -> v1), varies x, y = y1
                // Edge 1: right  (v1 -> v2), varies y, x = x1
                // Edge 2: top    (v2 -> v3), varies x, y = y0  (reversed: v2 at x1, v3 at x0)
                // Edge 3: left   (v3 -> v0), varies y, x = x0  (reversed: v3 at y0, v0 at y1)
                let t0 = Self::interpolate_edge(v0, v1, level);
                let p0 = (x0 + t0 * (x1 - x0), y1); // bottom edge

                let t1 = Self::interpolate_edge(v1, v2, level);
                let p1 = (x1, y1 + t1 * (y0 - y1)); // right edge

                let t2 = Self::interpolate_edge(v2, v3, level);
                let p2 = (x1 + t2 * (x0 - x1), y0); // top edge (right to left)

                let t3 = Self::interpolate_edge(v3, v0, level);
                let p3 = (x0, y0 + t3 * (y1 - y0)); // left edge (top to bottom)

                let seg = |a: (f64, f64), b: (f64, f64)| LineSegment {
                    x0: a.0, y0: a.1, x1: b.0, y1: b.1,
                };

                // All 16 cases
                match case {
                    0 | 15 => {}
                    1  => segments.push(seg(p0, p3)),
                    2  => segments.push(seg(p0, p1)),
                    3  => segments.push(seg(p1, p3)),
                    4  => segments.push(seg(p1, p2)),
                    5  => {
                        // Ambiguous: resolve by center average
                        let center = (v0 + v1 + v2 + v3) / 4.0;
                        if center >= level {
                            segments.push(seg(p0, p1));
                            segments.push(seg(p2, p3));
                        } else {
                            segments.push(seg(p0, p3));
                            segments.push(seg(p1, p2));
                        }
                    }
                    6  => segments.push(seg(p0, p2)),
                    7  => segments.push(seg(p2, p3)),
                    8  => segments.push(seg(p2, p3)),
                    9  => segments.push(seg(p0, p2)),
                    10 => {
                        let center = (v0 + v1 + v2 + v3) / 4.0;
                        if center >= level {
                            segments.push(seg(p0, p3));
                            segments.push(seg(p1, p2));
                        } else {
                            segments.push(seg(p0, p1));
                            segments.push(seg(p2, p3));
                        }
                    }
                    11 => segments.push(seg(p1, p2)),
                    12 => segments.push(seg(p1, p3)),
                    13 => segments.push(seg(p0, p1)),
                    14 => segments.push(seg(p0, p3)),
                    _  => {}
                }
            }
        }
        segments
    }

    /// Extract contours at multiple levels with distinct colours.
    pub fn multi_level(field: &ScalarField2D, levels: &[f64]) -> Vec<ContourLevel> {
        let n = levels.len().max(1);
        levels
            .iter()
            .enumerate()
            .map(|(k, &level)| {
                let hue = k as f64 / n as f64;
                let color = hsv_to_rgb(hue, 0.8, 0.9);
                ContourLevel {
                    level,
                    segments: Self::extract(field, level),
                    color,
                }
            })
            .collect()
    }

    /// Rasterise contour segments into an RGB pixel grid.
    pub fn render_contours(
        field: &ScalarField2D,
        levels: &[f64],
        width: u32,
        height: u32,
    ) -> Vec<Vec<[u8; 3]>> {
        let w = width as usize;
        let h = height as usize;
        let mut pixels = vec![vec![[20u8, 20, 20]; w]; h];

        let contours = Self::multi_level(field, levels);

        let to_px = |x: f64, y: f64| -> (isize, isize) {
            let tx = (x - field.x_range.0) / (field.x_range.1 - field.x_range.0);
            let ty = (y - field.y_range.0) / (field.y_range.1 - field.y_range.0);
            let px = (tx * (w - 1) as f64).round() as isize;
            let py = (ty * (h - 1) as f64).round() as isize;
            (px, py)
        };

        for cl in &contours {
            for seg in &cl.segments {
                // Bresenham line rasterisation
                let (x0, y0) = to_px(seg.x0, seg.y0);
                let (x1, y1) = to_px(seg.x1, seg.y1);
                for (px, py) in bresenham(x0, y0, x1, y1) {
                    if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                        pixels[py as usize][px as usize] = cl.color;
                    }
                }
            }
        }
        pixels
    }

    /// Produce an ASCII art representation of the contours.
    pub fn to_ascii(
        segments: &[LineSegment],
        field: &ScalarField2D,
        width: u32,
        height: u32,
    ) -> Vec<Vec<char>> {
        let w = width as usize;
        let h = height as usize;
        let mut grid = vec![vec!['.'; w]; h];

        let to_px = |x: f64, y: f64| -> (isize, isize) {
            let tx = (x - field.x_range.0) / (field.x_range.1 - field.x_range.0);
            let ty = (y - field.y_range.0) / (field.y_range.1 - field.y_range.0);
            let px = (tx * (w - 1) as f64).round() as isize;
            let py = (ty * (h - 1) as f64).round() as isize;
            (px, py)
        };

        for seg in segments {
            let (x0, y0) = to_px(seg.x0, seg.y0);
            let (x1, y1) = to_px(seg.x1, seg.y1);
            let dx = (x1 - x0).abs();
            let dy = (y1 - y0).abs();
            let ch = if dx > dy { '-' } else { '|' };
            for (px, py) in bresenham(x0, y0, x1, y1) {
                if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                    grid[py as usize][px as usize] = ch;
                }
            }
        }
        grid
    }
}

// ---------------------------------------------------------------------------
// Common scalar fields
// ---------------------------------------------------------------------------

/// Circular level-set: `f(x, y) = x² + y²`.
pub fn circle_field(
    width: usize,
    height: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
) -> ScalarField2D {
    ScalarField2D::from_function(width, height, x_range, y_range, |x, y| x * x + y * y)
}

/// Torus-knot inspired field: uses trigonometric combinations.
pub fn torus_knot_field(
    width: usize,
    height: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
) -> ScalarField2D {
    use std::f64::consts::PI;
    ScalarField2D::from_function(width, height, x_range, y_range, |x, y| {
        let r = (x * x + y * y).sqrt();
        let theta = y.atan2(x);
        (r - 0.5).powi(2) + (0.2 * (3.0 * theta - 2.0 * PI * r).sin()).powi(2)
    })
}

/// Sine-wave field: `f(x, y) = sin(πx) * cos(πy)`.
pub fn sine_field(
    width: usize,
    height: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
) -> ScalarField2D {
    use std::f64::consts::PI;
    ScalarField2D::from_function(width, height, x_range, y_range, |x, y| {
        (PI * x).sin() * (PI * y).cos()
    })
}

/// Metaball field: sum of Gaussian blobs.
///
/// Each centre is `(cx, cy, radius)`.
pub fn metaball_field(
    width: usize,
    height: usize,
    x_range: (f64, f64),
    y_range: (f64, f64),
    centers: &[(f64, f64, f64)],
) -> ScalarField2D {
    let centers = centers.to_vec();
    ScalarField2D::from_function(width, height, x_range, y_range, move |x, y| {
        centers.iter().map(|&(cx, cy, r)| {
            let d2 = (x - cx).powi(2) + (y - cy).powi(2);
            let r2 = r * r;
            if r2 < 1e-12 { 0.0 } else { (-d2 / (2.0 * r2)).exp() }
        }).sum()
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as u32;
    let f = h - h.floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Bresenham line iterator.
fn bresenham(x0: isize, y0: isize, x1: isize, y1: isize) -> Vec<(isize, isize)> {
    let mut pts = Vec::new();
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: isize = if x0 < x1 { 1 } else { -1 };
    let sy: isize = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        pts.push((x, y));
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
    pts
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circle_field_center() {
        let f = circle_field(5, 5, (-1.0, 1.0), (-1.0, 1.0));
        let center = f.get(2, 2);
        assert!(center < 0.1, "center value should be ~0, got {}", center);
    }

    #[test]
    fn test_extract_circle_produces_segments() {
        let f = circle_field(50, 50, (-1.0, 1.0), (-1.0, 1.0));
        let segs = MarchingSquares::extract(&f, 0.25);
        assert!(!segs.is_empty(), "expected contour segments for circle at r=0.5");
    }

    #[test]
    fn test_bilinear_sample_corners() {
        let f = ScalarField2D::from_function(3, 3, (0.0, 1.0), (0.0, 1.0), |x, y| x + y);
        let v = f.sample_bilinear(0.0, 0.0);
        assert!(v.abs() < 0.01, "got {}", v);
        let v = f.sample_bilinear(1.0, 1.0);
        assert!((v - 2.0).abs() < 0.01, "got {}", v);
    }

    #[test]
    fn test_multi_level() {
        let f = circle_field(40, 40, (-1.0, 1.0), (-1.0, 1.0));
        let levels = [0.1, 0.4, 0.9];
        let contours = MarchingSquares::multi_level(&f, &levels);
        assert_eq!(contours.len(), 3);
    }

    #[test]
    fn test_render_contours_dimensions() {
        let f = sine_field(30, 30, (-1.0, 1.0), (-1.0, 1.0));
        let pixels = MarchingSquares::render_contours(&f, &[0.0], 64, 64);
        assert_eq!(pixels.len(), 64);
        assert_eq!(pixels[0].len(), 64);
    }

    #[test]
    fn test_metaball_field() {
        let centers = [(0.0, 0.0, 0.3), (0.5, 0.5, 0.2)];
        let f = metaball_field(30, 30, (-1.0, 1.0), (-1.0, 1.0), &centers);
        let segs = MarchingSquares::extract(&f, 0.5);
        // May or may not have segments depending on field values; just check no panic
        let _ = segs;
    }

    #[test]
    fn test_interpolate_edge() {
        assert!((MarchingSquares::interpolate_edge(0.0, 1.0, 0.5) - 0.5).abs() < 1e-9);
        assert!((MarchingSquares::interpolate_edge(0.0, 2.0, 1.0) - 0.5).abs() < 1e-9);
    }
}
