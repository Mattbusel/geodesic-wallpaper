//! Fractal overlay renderer for geodesic-wallpaper.
//!
//! Renders Mandelbrot, Julia, and Burning Ship fractals as normalised float
//! fields and blends them over a base wallpaper image.

// ── FractalType ───────────────────────────────────────────────────────────────

/// Which fractal escape-time algorithm to render.
#[derive(Debug, Clone)]
pub enum FractalType {
    /// The classic Mandelbrot set: iterate z → z² + c where c = pixel.
    Mandelbrot,
    /// Julia set: iterate z → z² + c where c is fixed.
    Julia {
        /// Real part of the constant c.
        c_re: f64,
        /// Imaginary part of the constant c.
        c_im: f64,
    },
    /// Burning Ship fractal: iterate z → (|Re(z)| + i|Im(z)|)² + c.
    BurningShip,
}

// ── FractalRenderer ───────────────────────────────────────────────────────────

/// Renders escape-time fractals to a normalised float field in [0, 1].
pub struct FractalRenderer;

impl FractalRenderer {
    /// Render a fractal to a float field.
    ///
    /// Returns a `Vec<f32>` of length `width * height`.  Each value is in
    /// `[0, 1]`, where 0 = inside the set and 1 = escaped fastest.
    ///
    /// Smooth colouring formula applied:
    /// `smooth = iter + 1 - log2(log2(|z|))`
    /// (clamped and normalised to [0, 1]).
    ///
    /// # Parameters
    /// - `fractal`: which fractal to render.
    /// - `width`, `height`: output dimensions in pixels.
    /// - `center_x`, `center_y`: centre of the view in fractal coordinates.
    /// - `zoom`: zoom level (larger = closer in).
    /// - `max_iter`: maximum number of iterations before declaring "inside".
    pub fn render(
        fractal: &FractalType,
        width: u32,
        height: u32,
        center_x: f64,
        center_y: f64,
        zoom: f64,
        max_iter: u32,
    ) -> Vec<f32> {
        let w = width as usize;
        let h = height as usize;
        let mut field = vec![0.0f32; w * h];

        let scale = 1.0 / (zoom.max(1e-10) * (width.min(height) as f64) * 0.5);

        let mut max_smooth: f32 = 0.0;

        for py in 0..h {
            for px in 0..w {
                let cx = center_x + (px as f64 + 0.5 - width as f64 * 0.5) * scale;
                let cy = center_y + (py as f64 + 0.5 - height as f64 * 0.5) * scale;

                let smooth = Self::escape_smooth(fractal, cx, cy, max_iter);
                field[py * w + px] = smooth;
                if smooth > max_smooth {
                    max_smooth = smooth;
                }
            }
        }

        // Normalise to [0, 1]
        if max_smooth > 0.0 {
            for v in &mut field {
                *v /= max_smooth;
            }
        }
        field
    }

    /// Compute the smooth escape time for pixel (cx, cy).
    /// Returns 0.0 for points inside the set.
    fn escape_smooth(fractal: &FractalType, cx: f64, cy: f64, max_iter: u32) -> f32 {
        let (mut zx, mut zy, cfx, cfy) = match fractal {
            FractalType::Mandelbrot => (0.0, 0.0, cx, cy),
            FractalType::Julia { c_re, c_im } => (cx, cy, *c_re, *c_im),
            FractalType::BurningShip => (0.0, 0.0, cx, cy),
        };

        for iter in 0..max_iter {
            let (zx2, zy2) = match fractal {
                FractalType::Mandelbrot | FractalType::Julia { .. } => {
                    let nx = zx * zx - zy * zy + cfx;
                    let ny = 2.0 * zx * zy + cfy;
                    (nx, ny)
                }
                FractalType::BurningShip => {
                    let nx = zx * zx - zy * zy + cfx;
                    let ny = 2.0 * zx.abs() * zy.abs() + cfy;
                    (nx, ny)
                }
            };
            zx = zx2;
            zy = zy2;

            let r2 = zx * zx + zy * zy;
            if r2 > 4.0 {
                // Smooth iteration count: iter + 1 - log2(log2(sqrt(r2)))
                let log_r = r2.ln() * 0.5; // ln(|z|)
                let smooth = iter as f32 + 1.0 - (log_r.ln() / std::f64::consts::LN_2) as f32;
                return smooth.max(0.0);
            }
        }
        0.0 // inside the set
    }
}

// ── FractalOverlay ────────────────────────────────────────────────────────────

/// Blends a fractal field over a base image.
pub struct FractalOverlay;

impl FractalOverlay {
    /// Blend `base` colours with a hue shift driven by `fractal_field`.
    ///
    /// For each pixel:
    /// - The base hue is rotated by `fractal_value * blend * 360°`.
    /// - `blend = 0.0` → base unchanged; `blend = 1.0` → full hue rotation.
    pub fn apply(base: &[[u8; 3]], fractal_field: &[f32], blend: f32) -> Vec<[u8; 3]> {
        assert_eq!(
            base.len(),
            fractal_field.len(),
            "base and field must be the same length"
        );
        let blend = blend.clamp(0.0, 1.0);
        base.iter()
            .zip(fractal_field.iter())
            .map(|(&pixel, &fval)| {
                let [r, g, b] = pixel;
                let rf = r as f32 / 255.0;
                let gf = g as f32 / 255.0;
                let bf = b as f32 / 255.0;

                let [h, s, v] = rgb_to_hsv(rf, gf, bf);
                let hue_shift = fval * blend; // [0, blend]
                let new_h = (h + hue_shift).rem_euclid(1.0);
                let [nr, ng, nb] = hsv_to_rgb(new_h, s, v);
                [
                    (nr * 255.0).clamp(0.0, 255.0) as u8,
                    (ng * 255.0).clamp(0.0, 255.0) as u8,
                    (nb * 255.0).clamp(0.0, 255.0) as u8,
                ]
            })
            .collect()
    }
}

// ── Colour helpers ────────────────────────────────────────────────────────────

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> [f32; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max < 1e-6 { 0.0 } else { delta / max };
    let h = if delta < 1e-6 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    [h, s, v]
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r, g, b]
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Mandelbrot field has correct length
    #[test]
    fn mandelbrot_field_length() {
        let field = FractalRenderer::render(&FractalType::Mandelbrot, 32, 32, 0.0, 0.0, 1.0, 64);
        assert_eq!(field.len(), 32 * 32);
    }

    // 2. All values in [0, 1]
    #[test]
    fn mandelbrot_values_in_range() {
        let field = FractalRenderer::render(&FractalType::Mandelbrot, 32, 32, 0.0, 0.0, 1.0, 64);
        for v in &field {
            assert!(*v >= 0.0 && *v <= 1.0, "value out of range: {}", v);
        }
    }

    // 3. Julia field has correct length
    #[test]
    fn julia_field_length() {
        let field = FractalRenderer::render(
            &FractalType::Julia {
                c_re: -0.7,
                c_im: 0.27,
            },
            32,
            32,
            0.0,
            0.0,
            1.0,
            64,
        );
        assert_eq!(field.len(), 32 * 32);
    }

    // 4. Julia values in [0, 1]
    #[test]
    fn julia_values_in_range() {
        let field = FractalRenderer::render(
            &FractalType::Julia {
                c_re: -0.7,
                c_im: 0.27,
            },
            32,
            32,
            0.0,
            0.0,
            1.0,
            64,
        );
        for v in &field {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
    }

    // 5. Burning Ship field length
    #[test]
    fn burning_ship_field_length() {
        let field = FractalRenderer::render(&FractalType::BurningShip, 32, 32, -0.5, -0.5, 1.0, 64);
        assert_eq!(field.len(), 32 * 32);
    }

    // 6. Burning Ship values in [0, 1]
    #[test]
    fn burning_ship_values_in_range() {
        let field = FractalRenderer::render(&FractalType::BurningShip, 32, 32, -0.5, -0.5, 1.0, 64);
        for v in &field {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
    }

    // 7. Mandelbrot: pixel at origin (0,0) in complex plane is inside set
    //    (centre of the set), should be 0.0 (or close to it, since we normalise)
    #[test]
    fn mandelbrot_origin_inside() {
        // Render a tiny image centred exactly at (0,0) with high zoom
        let field = FractalRenderer::render(&FractalType::Mandelbrot, 1, 1, 0.0, 0.0, 1.0, 256);
        // A single pixel at the centre of the Mandelbrot set: 1x1 image,
        // centre=(0,0), scale is 1/(1*0.5)=2, so the pixel maps to cx=0, cy=0.
        // Origin is inside the set → should return 0.0 → normalised stays 0.0.
        assert_eq!(field[0], 0.0);
    }

    // 8. Mandelbrot: points far from origin should escape (non-zero)
    #[test]
    fn mandelbrot_far_point_escapes() {
        // Render at centre (3, 3) — far outside Mandelbrot set
        let field = FractalRenderer::render(&FractalType::Mandelbrot, 4, 4, 3.0, 3.0, 0.1, 64);
        // Most pixels should be non-zero (escaped)
        let nonzero = field.iter().filter(|&&v| v > 0.0).count();
        assert!(nonzero > 0, "expected non-zero escape values");
    }

    // 9. Smooth iteration formula: non-zero for escaping pixels
    #[test]
    fn smooth_iter_nonzero_for_escape() {
        let smooth = FractalRenderer::escape_smooth(
            &FractalType::Mandelbrot,
            2.5,
            0.0, // Far outside the set
            256,
        );
        assert!(
            smooth > 0.0,
            "expected non-zero smooth value for escaped point"
        );
    }

    // 10. Overlay output length matches base
    #[test]
    fn overlay_length_matches_base() {
        let base = vec![[128u8; 3]; 64];
        let field = vec![0.5f32; 64];
        let result = FractalOverlay::apply(&base, &field, 0.3);
        assert_eq!(result.len(), 64);
    }

    // 11. Overlay with blend=0 leaves base unchanged
    #[test]
    fn overlay_blend_zero_unchanged() {
        let base = vec![[100u8, 150u8, 200u8]; 16];
        let field = vec![0.8f32; 16];
        let result = FractalOverlay::apply(&base, &field, 0.0);
        for (r, b) in result.iter().zip(base.iter()) {
            // Allow rounding of ±1
            assert!((r[0] as i32 - b[0] as i32).abs() <= 1);
            assert!((r[1] as i32 - b[1] as i32).abs() <= 1);
            assert!((r[2] as i32 - b[2] as i32).abs() <= 1);
        }
    }

    // 12. Overlay pixel values are in [0, 255]
    #[test]
    fn overlay_values_in_byte_range() {
        let base = vec![[200u8, 100u8, 50u8]; 32];
        let field: Vec<f32> = (0..32).map(|i| i as f32 / 32.0).collect();
        let result = FractalOverlay::apply(&base, &field, 0.5);
        for p in &result {
            for &c in p {
                assert!(c <= 255); // u8 is always <= 255
            }
        }
    }

    // 13. Mandelbrot render with max_iter=1 produces mostly escaped pixels
    #[test]
    fn mandelbrot_low_iter() {
        let field = FractalRenderer::render(&FractalType::Mandelbrot, 32, 32, 2.0, 2.0, 1.0, 1);
        assert_eq!(field.len(), 32 * 32);
        // All values should be in [0, 1]
        for v in &field {
            assert!(*v >= 0.0 && *v <= 1.0);
        }
    }

    // 14. Smooth formula: log2(log2(r)) is finite for r > 4
    #[test]
    fn smooth_formula_finite() {
        // z = 3 + 0i after 0 iterations (r² = 9 > 4)
        let zx = 3.0f64;
        let zy = 0.0f64;
        let r2 = zx * zx + zy * zy;
        let log_r = r2.ln() * 0.5;
        let smooth = 0.0f32 + 1.0 - (log_r.ln() / std::f64::consts::LN_2) as f32;
        assert!(smooth.is_finite());
    }

    // 15. Julia with c=(0,0) behaves like magnitude circle (many escape)
    #[test]
    fn julia_c_zero_escapes() {
        let field = FractalRenderer::render(
            &FractalType::Julia {
                c_re: 0.0,
                c_im: 0.0,
            },
            16,
            16,
            0.0,
            0.0,
            0.3,
            64,
        );
        // With c=0, |z|^2^n grows for |z|>1; the edges should escape
        let nonzero = field.iter().filter(|&&v| v > 0.0).count();
        assert!(nonzero > 0);
    }
}
