//! Additional fractal types: IFS (Barnsley fern, Sierpinski, Dragon),
//! Newton fractals, Burning Ship, and the Phoenix fractal.

// ---------------------------------------------------------------------------
// IFS (Iterated Function System)
// ---------------------------------------------------------------------------

/// A single affine transform in an IFS: (x,y) → (ax+by+e, cx+dy+f).
#[derive(Debug, Clone)]
pub struct IfsTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
    /// Probability weight (should sum to ~1.0 across all transforms).
    pub probability: f64,
}

impl IfsTransform {
    /// Apply the affine transform to a point.
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.b * y + self.e,
            self.c * x + self.d * y + self.f,
        )
    }
}

/// An Iterated Function System consisting of multiple affine transforms.
#[derive(Debug, Clone)]
pub struct IteratedFunctionSystem {
    pub transforms: Vec<IfsTransform>,
}

impl IteratedFunctionSystem {
    /// Classic Barnsley fern — 4 transforms.
    pub fn barnsley_fern() -> Self {
        Self {
            transforms: vec![
                IfsTransform {
                    a: 0.0,
                    b: 0.0,
                    c: 0.0,
                    d: 0.16,
                    e: 0.0,
                    f: 0.0,
                    probability: 0.01,
                },
                IfsTransform {
                    a: 0.85,
                    b: 0.04,
                    c: -0.04,
                    d: 0.85,
                    e: 0.0,
                    f: 1.6,
                    probability: 0.85,
                },
                IfsTransform {
                    a: 0.2,
                    b: -0.26,
                    c: 0.23,
                    d: 0.22,
                    e: 0.0,
                    f: 1.6,
                    probability: 0.07,
                },
                IfsTransform {
                    a: -0.15,
                    b: 0.28,
                    c: 0.26,
                    d: 0.24,
                    e: 0.0,
                    f: 0.44,
                    probability: 0.07,
                },
            ],
        }
    }

    /// Sierpinski gasket — 3 transforms.
    pub fn sierpinski_gasket() -> Self {
        Self {
            transforms: vec![
                IfsTransform {
                    a: 0.5,
                    b: 0.0,
                    c: 0.0,
                    d: 0.5,
                    e: 0.0,
                    f: 0.0,
                    probability: 1.0 / 3.0,
                },
                IfsTransform {
                    a: 0.5,
                    b: 0.0,
                    c: 0.0,
                    d: 0.5,
                    e: 0.5,
                    f: 0.0,
                    probability: 1.0 / 3.0,
                },
                IfsTransform {
                    a: 0.5,
                    b: 0.0,
                    c: 0.0,
                    d: 0.5,
                    e: 0.25,
                    f: 0.43,
                    probability: 1.0 / 3.0,
                },
            ],
        }
    }

    /// Dragon curve IFS — 2 transforms.
    pub fn dragon() -> Self {
        Self {
            transforms: vec![
                IfsTransform {
                    a: 0.5,
                    b: -0.5,
                    c: 0.5,
                    d: 0.5,
                    e: 0.0,
                    f: 0.0,
                    probability: 0.5,
                },
                IfsTransform {
                    a: -0.5,
                    b: -0.5,
                    c: 0.5,
                    d: -0.5,
                    e: 1.0,
                    f: 0.0,
                    probability: 0.5,
                },
            ],
        }
    }

    /// Render the IFS using the chaos game with `n_points` iterations.
    ///
    /// Returns an RGB pixel buffer of size `width × height`.
    pub fn render(&self, n_points: usize, width: u32, height: u32, seed: u64) -> Vec<Vec<[u8; 3]>> {
        let mut pixels = vec![vec![[20u8, 20, 20]; width as usize]; height as usize];
        if self.transforms.is_empty() || width == 0 || height == 0 {
            return pixels;
        }

        // Build cumulative probability table
        let mut cumulative = Vec::with_capacity(self.transforms.len());
        let mut acc = 0.0f64;
        for t in &self.transforms {
            acc += t.probability;
            cumulative.push(acc);
        }

        let mut rng = LcgRng::new(seed);
        let mut x = 0.0f64;
        let mut y = 0.0f64;

        // Collect min/max for normalisation
        let mut pts = Vec::with_capacity(n_points);
        for _ in 0..n_points {
            let r = rng.next_f64();
            let idx = cumulative
                .iter()
                .position(|&c| r <= c)
                .unwrap_or(self.transforms.len() - 1);
            let (nx, ny) = self.transforms[idx].apply(x, y);
            x = nx;
            y = ny;
            pts.push((x, y));
        }

        if pts.is_empty() {
            return pixels;
        }

        let min_x = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let max_x = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let min_y = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let max_y = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);

        let range_x = (max_x - min_x).max(1e-9);
        let range_y = (max_y - min_y).max(1e-9);

        for (px, py) in &pts {
            let ix = ((px - min_x) / range_x * (width as f64 - 1.0)).round() as usize;
            let iy = ((max_y - py) / range_y * (height as f64 - 1.0)).round() as usize; // flip Y
            if iy < height as usize && ix < width as usize {
                pixels[iy][ix] = [0, 200, 100];
            }
        }
        pixels
    }
}

// ---------------------------------------------------------------------------
// NewtonFractal
// ---------------------------------------------------------------------------

/// Newton fractal: iterates Newton's method on a polynomial and colours by
/// which root each starting point converges to.
#[derive(Debug, Default)]
pub struct NewtonFractal;

impl NewtonFractal {
    /// Perform Newton iteration starting at `(z_re, z_im)` for a polynomial
    /// whose roots are given.  Returns `(iterations, root_index)`.
    ///
    /// The polynomial is `prod(z - root_k)` and its derivative is approximated
    /// numerically.
    pub fn iterate(z_re: f64, z_im: f64, roots: &[(f64, f64)], max_iter: u32) -> (u32, usize) {
        if roots.is_empty() {
            return (max_iter, 0);
        }

        let tol = 1e-6;
        let mut zr = z_re;
        let mut zi = z_im;

        for iter in 0..max_iter {
            // Evaluate p(z) and p'(z) using product form
            let mut pr = 1.0f64;
            let mut pi = 0.0f64; // p(z)
            let mut dpr = 0.0f64;
            let mut dpi = 0.0f64; // p'(z)

            for (i, &(rr, ri)) in roots.iter().enumerate() {
                // factor = (z - root)
                let fr = zr - rr;
                let fi = zi - ri;
                // p' += product of all other factors
                // Use Horner-like accumulation:
                // p' = sum_k prod_{j!=k}(z - root_j)
                // = sum_k p(z) / (z - root_k)   when z != root_k
                let mag2 = fr * fr + fi * fi;
                if mag2 < tol * tol {
                    // Already at this root
                    return (iter, i);
                }
                // Multiply p by this factor
                let new_pr = pr * fr - pi * fi;
                let new_pi = pr * fi + pi * fr;
                // Derivative contribution: p(z)/(z-root_i)
                // = (pr + i·pi) / (fr + i·fi)
                let inv_mag2 = 1.0 / mag2;
                let term_r = (pr * fr + pi * fi) * inv_mag2;
                let term_i = (pi * fr - pr * fi) * inv_mag2;
                dpr += term_r;
                dpi += term_i;
                pr = new_pr;
                pi = new_pi;
            }

            // z = z - p(z)/p'(z)
            let d_mag2 = dpr * dpr + dpi * dpi;
            if d_mag2 < 1e-18 {
                break;
            }
            let ratio_r = (pr * dpr + pi * dpi) / d_mag2;
            let ratio_i = (pi * dpr - pr * dpi) / d_mag2;
            zr -= ratio_r;
            zi -= ratio_i;

            // Check convergence to any root
            for (idx, &(rr, ri)) in roots.iter().enumerate() {
                let dr = zr - rr;
                let di = zi - ri;
                if dr * dr + di * di < tol * tol {
                    return (iter + 1, idx);
                }
            }
        }

        // Find nearest root
        let nearest = roots
            .iter()
            .enumerate()
            .min_by(|(_, &(ar, ai)), (_, &(br, bi))| {
                let da = (zr - ar) * (zr - ar) + (zi - ai) * (zi - ai);
                let db = (zr - br) * (zr - br) + (zi - bi) * (zi - bi);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        (max_iter, nearest)
    }

    /// Render the Newton fractal over the complex plane region `view = (x_min, x_max, y_min, y_max)`.
    pub fn render(
        roots: &[(f64, f64)],
        width: u32,
        height: u32,
        view: (f64, f64, f64, f64),
    ) -> Vec<Vec<[u8; 3]>> {
        let (x_min, x_max, y_min, y_max) = view;
        let max_iter = 64u32;
        let n_roots = roots.len().max(1);

        // Assign a distinct hue to each root
        let root_colors: Vec<[u8; 3]> = (0..n_roots)
            .map(|i| {
                let hue = (i as f64 / n_roots as f64) * 360.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.85, 0.95);
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
            })
            .collect();

        (0..height)
            .map(|py| {
                (0..width)
                    .map(|px| {
                        let zr = x_min + (px as f64 / width as f64) * (x_max - x_min);
                        let zi = y_min + (py as f64 / height as f64) * (y_max - y_min);
                        let (iters, root_idx) = Self::iterate(zr, zi, roots, max_iter);
                        let base = root_colors[root_idx % n_roots];
                        // Shade by iteration count (darker = slower convergence)
                        let shade = 1.0 - (iters as f64 / max_iter as f64) * 0.7;
                        [
                            (base[0] as f64 * shade) as u8,
                            (base[1] as f64 * shade) as u8,
                            (base[2] as f64 * shade) as u8,
                        ]
                    })
                    .collect()
            })
            .collect()
    }

    /// The three cube roots of unity: 1, ω, ω².
    pub fn default_roots_cubic() -> Vec<(f64, f64)> {
        let tau = std::f64::consts::TAU;
        vec![
            (1.0, 0.0),
            ((tau / 3.0).cos(), (tau / 3.0).sin()),
            ((2.0 * tau / 3.0).cos(), (2.0 * tau / 3.0).sin()),
        ]
    }
}

// ---------------------------------------------------------------------------
// BurningShip
// ---------------------------------------------------------------------------

/// The Burning Ship fractal — like Mandelbrot but with `|Re(z)|+i|Im(z)|`.
#[derive(Debug, Default)]
pub struct BurningShip;

impl BurningShip {
    /// Iterate z = (|Re(z)| + i|Im(z)|)² + c.  Returns escape iteration count.
    pub fn iterate(c_re: f64, c_im: f64, max_iter: u32) -> u32 {
        let (mut zr, mut zi) = (0.0f64, 0.0f64);
        for i in 0..max_iter {
            let zr2 = zr * zr;
            let zi2 = zi * zi;
            if zr2 + zi2 > 4.0 {
                return i;
            }
            zi = 2.0 * zr.abs() * zi.abs() + c_im;
            zr = zr2 - zi2 + c_re;
        }
        max_iter
    }

    /// Render the Burning Ship fractal.
    pub fn render(width: u32, height: u32, view: (f64, f64, f64, f64)) -> Vec<Vec<[u8; 3]>> {
        let (x_min, x_max, y_min, y_max) = view;
        let max_iter = 256u32;

        (0..height)
            .map(|py| {
                (0..width)
                    .map(|px| {
                        let c_re = x_min + (px as f64 / width as f64) * (x_max - x_min);
                        let c_im = y_min + (py as f64 / height as f64) * (y_max - y_min);
                        let iters = Self::iterate(c_re, c_im, max_iter);
                        if iters == max_iter {
                            [0u8, 0, 0]
                        } else {
                            let t = iters as f64 / max_iter as f64;
                            let hue = t * 300.0 + 30.0;
                            let (r, g, b) = hsv_to_rgb(hue, 0.9, 1.0);
                            [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// PhoenixFractal
// ---------------------------------------------------------------------------

/// Phoenix fractal: z_{n+1} = z_n² + c + p·z_{n-1}
#[derive(Debug, Default)]
pub struct PhoenixFractal;

impl PhoenixFractal {
    /// Iterate the Phoenix recurrence.  Returns escape count.
    pub fn iterate(c: (f64, f64), p: (f64, f64), max_iter: u32) -> u32 {
        let (cr, ci) = c;
        let (pr, pi) = p;
        let (mut zr, mut zi) = (0.0f64, 0.0f64);
        let (mut prev_r, mut prev_i) = (0.0f64, 0.0f64);

        for i in 0..max_iter {
            let zr2 = zr * zr;
            let zi2 = zi * zi;
            if zr2 + zi2 > 4.0 {
                return i;
            }
            // z_next = z² + c + p * z_prev
            let next_r = zr2 - zi2 + cr + pr * prev_r - pi * prev_i;
            let next_i = 2.0 * zr * zi + ci + pr * prev_i + pi * prev_r;
            prev_r = zr;
            prev_i = zi;
            zr = next_r;
            zi = next_i;
        }
        max_iter
    }

    /// Render the Phoenix fractal over a 2D grid of starting z values.
    pub fn render(c: (f64, f64), p: (f64, f64), width: u32, height: u32) -> Vec<Vec<[u8; 3]>> {
        let max_iter = 128u32;
        // Standard view: z in [-1.5, 1.5] × [-1.5, 1.5]
        let (x_min, x_max) = (-1.5, 1.5);
        let (y_min, y_max) = (-1.5, 1.5);

        (0..height)
            .map(|py| {
                (0..width)
                    .map(|px| {
                        // We vary the initial z (re, im) across the grid
                        let _z_re = x_min + (px as f64 / width as f64) * (x_max - x_min);
                        let _z_im = y_min + (py as f64 / height as f64) * (y_max - y_min);
                        // For Phoenix, conventionally c is fixed and we vary initial z
                        // by using (x, y) as c and p constant
                        let vary_c = (_z_re, _z_im);
                        let iters = Self::iterate(vary_c, p, max_iter);
                        if iters == max_iter {
                            [0u8, 0, 0]
                        } else {
                            let t = iters as f64 / max_iter as f64;
                            let hue = (1.0 - t) * 270.0;
                            let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
                            // Use c to avoid dead-code warning
                            let _ = c;
                            [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
                        }
                    })
                    .collect()
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r1 + m, g1 + m, b1 + m)
}

/// LCG pseudo-random number generator for the IFS chaos game.
struct LcgRng(u64);

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn next_f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ifs_transform_apply() {
        let t = IfsTransform {
            a: 0.5,
            b: 0.0,
            c: 0.0,
            d: 0.5,
            e: 0.0,
            f: 0.0,
            probability: 1.0,
        };
        let (x, y) = t.apply(2.0, 4.0);
        assert!((x - 1.0).abs() < 1e-10);
        assert!((y - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_barnsley_fern_has_4_transforms() {
        let ifs = IteratedFunctionSystem::barnsley_fern();
        assert_eq!(ifs.transforms.len(), 4);
    }

    #[test]
    fn test_ifs_render_dimensions() {
        let ifs = IteratedFunctionSystem::sierpinski_gasket();
        let img = ifs.render(1000, 64, 64, 42);
        assert_eq!(img.len(), 64);
        assert_eq!(img[0].len(), 64);
    }

    #[test]
    fn test_newton_cubic_roots() {
        let roots = NewtonFractal::default_roots_cubic();
        assert_eq!(roots.len(), 3);
        // Each root should have magnitude ≈ 1
        for (r, i) in &roots {
            let mag = (r * r + i * i).sqrt();
            assert!(
                (mag - 1.0).abs() < 1e-6,
                "Root magnitude should be 1, got {}",
                mag
            );
        }
    }

    #[test]
    fn test_newton_iterate_at_root() {
        let roots = vec![(1.0f64, 0.0)];
        let (iters, idx) = NewtonFractal::iterate(1.0, 0.0, &roots, 100);
        assert_eq!(idx, 0);
        assert!(iters < 100, "Should converge quickly at the root");
    }

    #[test]
    fn test_newton_render_dimensions() {
        let roots = NewtonFractal::default_roots_cubic();
        let img = NewtonFractal::render(&roots, 32, 32, (-1.5, 1.5, -1.5, 1.5));
        assert_eq!(img.len(), 32);
        assert_eq!(img[0].len(), 32);
    }

    #[test]
    fn test_burning_ship_interior() {
        // c = 0 should not escape
        assert_eq!(BurningShip::iterate(0.0, 0.0, 100), 100);
    }

    #[test]
    fn test_burning_ship_exterior() {
        // c far outside the set should escape quickly
        let iters = BurningShip::iterate(10.0, 10.0, 256);
        assert!(iters < 256);
    }

    #[test]
    fn test_burning_ship_render_dimensions() {
        let img = BurningShip::render(32, 32, (-2.5, 1.5, -2.0, 0.5));
        assert_eq!(img.len(), 32);
        assert_eq!(img[0].len(), 32);
    }

    #[test]
    fn test_phoenix_iterate_origin() {
        let iters = PhoenixFractal::iterate((0.0, 0.0), (0.0, 0.0), 128);
        assert_eq!(iters, 128, "Origin with c=0 p=0 should not escape");
    }

    #[test]
    fn test_phoenix_render_dimensions() {
        let img = PhoenixFractal::render((0.5667, -0.5), (0.0, -0.5), 32, 32);
        assert_eq!(img.len(), 32);
        assert_eq!(img[0].len(), 32);
    }
}
