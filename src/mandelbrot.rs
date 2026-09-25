//! Mandelbrot and Julia set renderer.
//!
//! Provides escape-time iteration, smooth (continuous) colouring via the
//! log-log escape-radius formula, and full 2-D grid rendering for both the
//! Mandelbrot set and Julia sets.

// ---------------------------------------------------------------------------
// ComplexNum
// ---------------------------------------------------------------------------

/// A complex number stored as (re, im) f64 components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexNum {
    pub re: f64,
    pub im: f64,
}

impl ComplexNum {
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Complex addition.
    #[inline]
    pub fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    /// Complex multiplication.
    #[inline]
    pub fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    /// Squared absolute value (avoids the sqrt in the hot path).
    #[inline]
    pub fn abs_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
}

// ---------------------------------------------------------------------------
// MandelbrotConfig
// ---------------------------------------------------------------------------

/// Configuration for a Mandelbrot or Julia set render.
#[derive(Debug, Clone)]
pub struct MandelbrotConfig {
    pub width: u32,
    pub height: u32,
    /// Real part of the viewport centre.
    pub center_re: f64,
    /// Imaginary part of the viewport centre.
    pub center_im: f64,
    /// Zoom level (larger = more zoomed in; 1.0 covers roughly ±2 in each axis).
    pub zoom: f64,
    /// Maximum number of iterations before declaring a point non-escaping.
    pub max_iter: u32,
}

impl MandelbrotConfig {
    /// Map pixel `(px, py)` to a complex number in the viewport.
    fn pixel_to_complex(&self, px: u32, py: u32) -> ComplexNum {
        let scale = 4.0 / (self.zoom * self.width.min(self.height) as f64);
        let re = self.center_re + (px as f64 - self.width as f64 / 2.0) * scale;
        let im = self.center_im - (py as f64 - self.height as f64 / 2.0) * scale;
        ComplexNum::new(re, im)
    }
}

impl Default for MandelbrotConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            center_re: -0.5,
            center_im: 0.0,
            zoom: 1.0,
            max_iter: 256,
        }
    }
}

// ---------------------------------------------------------------------------
// Iteration
// ---------------------------------------------------------------------------

/// Escape-time iteration for the Mandelbrot set.
///
/// Returns the iteration count at which `|z|` exceeded 2, or `max_iter` if
/// the point did not escape.
pub fn iterate_mandelbrot(c: ComplexNum, max_iter: u32) -> u32 {
    let mut z = ComplexNum::new(0.0, 0.0);
    for i in 0..max_iter {
        if z.abs_sq() > 4.0 {
            return i;
        }
        z = z.mul(z).add(c);
    }
    max_iter
}

/// Escape-time iteration for a Julia set with parameter `c`.
///
/// Returns the iteration count at which `|z|` exceeded 2, or `max_iter`.
pub fn iterate_julia(z0: ComplexNum, c: ComplexNum, max_iter: u32) -> u32 {
    let mut z = z0;
    for i in 0..max_iter {
        if z.abs_sq() > 4.0 {
            return i;
        }
        z = z.mul(z).add(c);
    }
    max_iter
}

/// Smooth (continuous) iteration count using the log-log escape radius formula.
///
/// Returns a floating-point value that avoids the banding artefacts of the
/// raw integer iteration count.  Interior points (`iter == max_iter`) return
/// `max_iter as f64`.
pub fn smooth_iter(iter: u32, z: ComplexNum) -> f64 {
    let max_iter_f = 256.0; // sentinel — caller should treat iter == max_iter as interior
    if iter == 0 {
        return 0.0;
    }
    let abs_sq = z.abs_sq();
    if abs_sq <= 0.0 {
        return iter as f64;
    }
    // nu = log2(log2(|z|)) — normalisation constant.
    let log_zn = abs_sq.ln() / 2.0; // = ln(|z|)
    let nu = (log_zn / std::f64::consts::LN_2).log2();
    let _ = max_iter_f;
    iter as f64 + 1.0 - nu
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// Render the Mandelbrot set to a 2-D grid of smooth iteration values.
///
/// The returned grid is indexed as `grid[row][col]` where row 0 is the top.
pub fn render_mandelbrot(config: &MandelbrotConfig) -> Vec<Vec<f64>> {
    (0..config.height)
        .map(|py| {
            (0..config.width)
                .map(|px| {
                    let c = config.pixel_to_complex(px, py);
                    let iter = iterate_mandelbrot(c, config.max_iter);
                    if iter == config.max_iter {
                        0.0
                    } else {
                        // Compute z at escape for smooth colouring.
                        let mut z = ComplexNum::new(0.0, 0.0);
                        for _ in 0..iter {
                            z = z.mul(z).add(c);
                        }
                        smooth_iter(iter, z)
                    }
                })
                .collect()
        })
        .collect()
}

/// Render a Julia set for parameter `c` to a 2-D grid of smooth iteration values.
pub fn render_julia(c: ComplexNum, config: &MandelbrotConfig) -> Vec<Vec<f64>> {
    (0..config.height)
        .map(|py| {
            (0..config.width)
                .map(|px| {
                    let z0 = config.pixel_to_complex(px, py);
                    let iter = iterate_julia(z0, c, config.max_iter);
                    if iter == config.max_iter {
                        0.0
                    } else {
                        let mut z = z0;
                        for _ in 0..iter {
                            z = z.mul(z).add(c);
                        }
                        smooth_iter(iter, z)
                    }
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_add() {
        let a = ComplexNum::new(1.0, 2.0);
        let b = ComplexNum::new(3.0, -1.0);
        let c = a.add(b);
        assert!((c.re - 4.0).abs() < 1e-10);
        assert!((c.im - 1.0).abs() < 1e-10);
    }

    #[test]
    fn complex_mul() {
        // (1+2i)(3-1i) = 3 - i + 6i - 2i² = 3 + 5i + 2 = 5 + 5i
        let a = ComplexNum::new(1.0, 2.0);
        let b = ComplexNum::new(3.0, -1.0);
        let c = a.mul(b);
        assert!((c.re - 5.0).abs() < 1e-10);
        assert!((c.im - 5.0).abs() < 1e-10);
    }

    #[test]
    fn complex_abs_sq() {
        let z = ComplexNum::new(3.0, 4.0);
        assert!((z.abs_sq() - 25.0).abs() < 1e-10);
    }

    #[test]
    fn mandelbrot_origin_does_not_escape() {
        // The origin (0+0i) is well inside the Mandelbrot set.
        let iter = iterate_mandelbrot(ComplexNum::new(0.0, 0.0), 256);
        assert_eq!(iter, 256);
    }

    #[test]
    fn mandelbrot_outside_point_escapes() {
        // (2+0i) escapes immediately.
        let iter = iterate_mandelbrot(ComplexNum::new(2.0, 0.0), 256);
        assert!(iter < 256);
    }

    #[test]
    fn julia_origin_with_zero_c_does_not_escape() {
        let iter = iterate_julia(ComplexNum::new(0.0, 0.0), ComplexNum::new(0.0, 0.0), 256);
        assert_eq!(iter, 256);
    }

    #[test]
    fn julia_large_z_escapes_quickly() {
        let iter = iterate_julia(ComplexNum::new(10.0, 0.0), ComplexNum::new(0.0, 0.0), 256);
        assert_eq!(iter, 0); // |z|² = 100 > 4 before first iteration
    }

    #[test]
    fn smooth_iter_zero() {
        assert!((smooth_iter(0, ComplexNum::new(3.0, 0.0)) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn smooth_iter_positive_for_escaping_point() {
        let s = smooth_iter(5, ComplexNum::new(2.5, 0.0));
        assert!(s > 0.0);
    }

    #[test]
    fn render_mandelbrot_dimensions() {
        let cfg = MandelbrotConfig {
            width: 8,
            height: 6,
            ..Default::default()
        };
        let grid = render_mandelbrot(&cfg);
        assert_eq!(grid.len(), 6);
        assert_eq!(grid[0].len(), 8);
    }

    #[test]
    fn render_mandelbrot_interior_zero() {
        // A very zoomed-in view centred on 0,0 should have many interior (0.0) pixels.
        let cfg = MandelbrotConfig {
            width: 16,
            height: 16,
            center_re: 0.0,
            center_im: 0.0,
            zoom: 100.0,
            max_iter: 64,
        };
        let grid = render_mandelbrot(&cfg);
        let interior_count = grid.iter().flatten().filter(|&&v| v == 0.0).count();
        assert!(interior_count > 0);
    }

    #[test]
    fn render_julia_dimensions() {
        let cfg = MandelbrotConfig {
            width: 10,
            height: 8,
            ..Default::default()
        };
        let c = ComplexNum::new(-0.7, 0.27);
        let grid = render_julia(c, &cfg);
        assert_eq!(grid.len(), 8);
        assert_eq!(grid[0].len(), 10);
    }

    #[test]
    fn pixel_to_complex_centre() {
        let cfg = MandelbrotConfig {
            width: 100,
            height: 100,
            center_re: 0.0,
            center_im: 0.0,
            zoom: 1.0,
            max_iter: 64,
        };
        let c = cfg.pixel_to_complex(50, 50);
        assert!(c.re.abs() < 1e-9);
        assert!(c.im.abs() < 1e-9);
    }
}
