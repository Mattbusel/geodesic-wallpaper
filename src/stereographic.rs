//! Stereographic projection and Möbius / circle-inversion conformal maps.
//!
//! The [`StereographicRenderer`] can draw latitude lines, longitude lines and
//! circle-inversion patterns onto a flat pixel buffer using the
//! stereographic projection from the unit sphere.

#![allow(dead_code)]

use std::f64::consts::{PI, TAU};

// ── StereographicConfig ───────────────────────────────────────────────────────

/// Configuration for the stereographic projection.
#[derive(Debug, Clone)]
pub struct StereographicConfig {
    /// Radius of the reference sphere.
    pub sphere_radius: f64,
    /// z-coordinate of the projection plane (usually 0.0 or −sphere_radius).
    pub projection_plane_z: f64,
    /// Centre of the output image in projection-plane coordinates (x, y).
    pub center: (f64, f64),
    /// Width of the output pixel buffer.
    pub width: u32,
    /// Height of the output pixel buffer.
    pub height: u32,
}

impl Default for StereographicConfig {
    fn default() -> Self {
        StereographicConfig {
            sphere_radius: 1.0,
            projection_plane_z: 0.0,
            center: (0.0, 0.0),
            width: 512,
            height: 512,
        }
    }
}

// ── Projection functions ───────────────────────────────────────────────────────

/// Project a sphere point `(theta, phi)` (spherical coords, radians) onto the
/// plane using stereographic projection from the north pole.
///
/// `theta` — polar angle from north pole [0, π].
/// `phi`   — azimuthal angle [0, 2π].
///
/// Returns `None` for the north pole itself (theta ≈ 0) where the projection
/// diverges.
pub fn sphere_to_plane(theta: f64, phi: f64, config: &StereographicConfig) -> Option<(f64, f64)> {
    // Cartesian sphere point:
    let r = config.sphere_radius;
    let x = r * theta.sin() * phi.cos();
    let y = r * theta.sin() * phi.sin();
    let z = r * theta.cos();

    // Stereographic projection from north pole (0, 0, r):
    let denom = r - z;
    if denom.abs() < 1e-12 {
        return None; // north pole
    }
    let proj_x = r * x / denom;
    let proj_y = r * y / denom;
    Some((proj_x, proj_y))
}

/// Map a plane point `(x, y)` back to a point on the sphere.
///
/// Returns `(sx, sy, sz)` — Cartesian coordinates on the sphere.
pub fn plane_to_sphere(x: f64, y: f64, config: &StereographicConfig) -> (f64, f64, f64) {
    let r = config.sphere_radius;
    let denom = r * r + x * x + y * y;
    let sx = 2.0 * r * r * x / denom;
    let sy = 2.0 * r * r * y / denom;
    let sz = r * (x * x + y * y - r * r) / denom;
    (sx, sy, sz)
}

// ── MobiusTransform ───────────────────────────────────────────────────────────

/// Möbius (fractional linear) transformation of the complex plane:
/// `f(z) = (az + b) / (cz + d)`
/// where all coefficients are complex numbers stored as `(real, imag)` pairs.
#[derive(Debug, Clone, PartialEq)]
pub struct MobiusTransform {
    pub a: (f64, f64),
    pub b: (f64, f64),
    pub c: (f64, f64),
    pub d: (f64, f64),
}

impl MobiusTransform {
    /// Identity transform: `f(z) = z`.
    pub fn identity() -> Self {
        MobiusTransform {
            a: (1.0, 0.0),
            b: (0.0, 0.0),
            c: (0.0, 0.0),
            d: (1.0, 0.0),
        }
    }

    /// Apply the Möbius transform to the complex number `z = z_re + i·z_im`.
    ///
    /// Returns the result as `(re, im)`.  Returns `(f64::INFINITY, 0.0)` if
    /// the denominator vanishes.
    pub fn apply(&self, z_re: f64, z_im: f64) -> (f64, f64) {
        // numerator: a*z + b
        let (a_re, a_im) = self.a;
        let (b_re, b_im) = self.b;
        let (c_re, c_im) = self.c;
        let (d_re, d_im) = self.d;

        let num_re = a_re * z_re - a_im * z_im + b_re;
        let num_im = a_re * z_im + a_im * z_re + b_im;

        let den_re = c_re * z_re - c_im * z_im + d_re;
        let den_im = c_re * z_im + c_im * z_re + d_im;

        let den_norm = den_re * den_re + den_im * den_im;
        if den_norm < 1e-30 {
            return (f64::INFINITY, 0.0);
        }
        let re = (num_re * den_re + num_im * den_im) / den_norm;
        let im = (num_im * den_re - num_re * den_im) / den_norm;
        (re, im)
    }

    /// Compose two Möbius transforms: `self ∘ other`, i.e., apply `other` first.
    pub fn compose(&self, other: &MobiusTransform) -> MobiusTransform {
        // (a₁z + b₁)/(c₁z + d₁) ∘ (a₂z + b₂)/(c₂z + d₂)
        // = ( a₁(a₂z+b₂)/(c₂z+d₂) + b₁ ) / ( c₁(a₂z+b₂)/(c₂z+d₂) + d₁ )
        // = ( a₁·(a₂z+b₂) + b₁·(c₂z+d₂) ) / ( c₁·(a₂z+b₂) + d₁·(c₂z+d₂) )
        // numerator: (a1*a2 + b1*c2)z + (a1*b2 + b1*d2)
        // denominator: (c1*a2 + d1*c2)z + (c1*b2 + d1*d2)
        let new_a = cmul(self.a, other.a);
        let new_a = cadd(new_a, cmul(self.b, other.c));

        let new_b = cmul(self.a, other.b);
        let new_b = cadd(new_b, cmul(self.b, other.d));

        let new_c = cmul(self.c, other.a);
        let new_c = cadd(new_c, cmul(self.d, other.c));

        let new_d = cmul(self.c, other.b);
        let new_d = cadd(new_d, cmul(self.d, other.d));

        MobiusTransform {
            a: new_a,
            b: new_b,
            c: new_c,
            d: new_d,
        }
    }
}

// ── StereographicRenderer ─────────────────────────────────────────────────────

/// Renders stereographic projection patterns into RGBA pixel buffers.
pub struct StereographicRenderer;

impl StereographicRenderer {
    /// Draw `n_lines` latitude circles onto a pixel buffer.
    ///
    /// Returns a `height × width` grid of RGB pixels (white background).
    pub fn render_latitude_lines(
        config: &StereographicConfig,
        n_lines: u32,
        color: [u8; 3],
    ) -> Vec<Vec<[u8; 3]>> {
        let mut buffer = white_buffer(config.width, config.height);
        let scale = config.width as f64 / 4.0;

        for k in 1..=n_lines {
            // Latitude circles: theta = k * PI / (n_lines + 1)
            let theta = k as f64 * PI / (n_lines + 1) as f64;
            let n_phi = 360u32;
            for j in 0..n_phi {
                let phi = j as f64 * TAU / n_phi as f64;
                if let Some((px, py)) = sphere_to_plane(theta, phi, config) {
                    let ix = ((px - config.center.0) * scale + config.width as f64 / 2.0) as i64;
                    let iy = ((py - config.center.1) * scale + config.height as f64 / 2.0) as i64;
                    if ix >= 0
                        && iy >= 0
                        && (ix as u32) < config.width
                        && (iy as u32) < config.height
                    {
                        buffer[iy as usize][ix as usize] = color;
                    }
                }
            }
        }
        buffer
    }

    /// Draw `n_lines` longitude (meridian) lines onto a pixel buffer.
    pub fn render_longitude_lines(
        config: &StereographicConfig,
        n_lines: u32,
        color: [u8; 3],
    ) -> Vec<Vec<[u8; 3]>> {
        let mut buffer = white_buffer(config.width, config.height);
        let scale = config.width as f64 / 4.0;

        for k in 0..n_lines {
            let phi = k as f64 * TAU / n_lines as f64;
            let n_theta = 180u32;
            for j in 1..n_theta {
                let theta = j as f64 * PI / n_theta as f64;
                if let Some((px, py)) = sphere_to_plane(theta, phi, config) {
                    let ix = ((px - config.center.0) * scale + config.width as f64 / 2.0) as i64;
                    let iy = ((py - config.center.1) * scale + config.height as f64 / 2.0) as i64;
                    if ix >= 0
                        && iy >= 0
                        && (ix as u32) < config.width
                        && (iy as u32) < config.height
                    {
                        buffer[iy as usize][ix as usize] = color;
                    }
                }
            }
        }
        buffer
    }

    /// Render circle inversion patterns in the unit circle.
    ///
    /// Each entry in `circles` is `(cx, cy, r)` defining a circle centre and
    /// radius in the projection plane.  Each circle is inverted in the unit
    /// circle via `z → 1 / conj(z)` and both the original and inverted circles
    /// are rasterised.
    pub fn render_circle_inversion(
        config: &StereographicConfig,
        circles: &[(f64, f64, f64)],
        color: [u8; 3],
    ) -> Vec<Vec<[u8; 3]>> {
        let mut buffer = white_buffer(config.width, config.height);
        let scale = config.width as f64 / 4.0;

        let draw = |buf: &mut Vec<Vec<[u8; 3]>>, cx: f64, cy: f64, r: f64| {
            let n_pts = 360u32;
            for k in 0..n_pts {
                let angle = k as f64 * TAU / n_pts as f64;
                let px = cx + r * angle.cos();
                let py = cy + r * angle.sin();
                let ix = (px * scale + config.width as f64 / 2.0) as i64;
                let iy = (py * scale + config.height as f64 / 2.0) as i64;
                if ix >= 0 && iy >= 0 && (ix as u32) < config.width && (iy as u32) < config.height {
                    buf[iy as usize][ix as usize] = color;
                }
            }
        };

        for &(cx, cy, r) in circles {
            draw(&mut buffer, cx, cy, r);

            // Invert in unit circle: z → 1/conj(z), applied to centre.
            // For a circle (cx,cy,r) inverted in |z|=1:
            //   d = cx*cx + cy*cy - r*r
            //   (only works when d ≠ 0; approximate for generality)
            let d = cx * cx + cy * cy - r * r;
            if d.abs() > 1e-9 {
                let inv_cx = cx / d;
                let inv_cy = cy / d;
                let inv_r = r / d.abs();
                draw(&mut buffer, inv_cx, inv_cy, inv_r);
            }
        }
        buffer
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Allocate a white (255,255,255) pixel buffer of dimensions width × height.
fn white_buffer(width: u32, height: u32) -> Vec<Vec<[u8; 3]>> {
    vec![vec![[255u8; 3]; width as usize]; height as usize]
}

/// Complex multiplication.
fn cmul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Complex addition.
fn cadd(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> StereographicConfig {
        StereographicConfig::default()
    }

    // ── Projection round-trips ────────────────────────────────────────────────

    #[test]
    fn test_south_pole_projects_to_origin() {
        let cfg = default_cfg(); // sphere_radius = 1
                                 // South pole: theta = PI, phi = 0.
        let result = sphere_to_plane(PI, 0.0, &cfg).unwrap();
        // The south pole projects to the origin (0, 0) for unit sphere.
        assert!(result.0.abs() < 1e-9, "x = {}", result.0);
        assert!(result.1.abs() < 1e-9, "y = {}", result.1);
    }

    #[test]
    fn test_north_pole_returns_none() {
        let cfg = default_cfg();
        // theta = 0 → north pole → projection diverges.
        assert!(sphere_to_plane(0.0, 0.0, &cfg).is_none());
    }

    #[test]
    fn test_plane_to_sphere_south_pole() {
        let cfg = default_cfg();
        // Origin in the plane maps to the south pole (0, 0, -1).
        let (sx, sy, sz) = plane_to_sphere(0.0, 0.0, &cfg);
        assert!(sx.abs() < 1e-9);
        assert!(sy.abs() < 1e-9);
        assert!((sz + 1.0).abs() < 1e-9, "sz = {}", sz);
    }

    #[test]
    fn test_round_trip_equator() {
        let cfg = default_cfg();
        // Equator: theta = PI/2.
        for k in 0..8u32 {
            let phi = k as f64 * TAU / 8.0;
            let (px, py) = sphere_to_plane(PI / 2.0, phi, &cfg).unwrap();
            let (sx, sy, sz) = plane_to_sphere(px, py, &cfg);
            // Should lie on the sphere: sx²+sy²+sz² ≈ 1.
            let r2 = sx * sx + sy * sy + sz * sz;
            assert!((r2 - 1.0).abs() < 1e-9, "r²={r2} for phi={phi}");
        }
    }

    // ── Möbius transform ──────────────────────────────────────────────────────

    #[test]
    fn test_identity_mobius() {
        let m = MobiusTransform::identity();
        let (re, im) = m.apply(3.0, 4.0);
        assert!((re - 3.0).abs() < 1e-12);
        assert!((im - 4.0).abs() < 1e-12);
    }

    #[test]
    fn test_mobius_inversion() {
        // f(z) = 1/z: a=0, b=1, c=1, d=0.
        let m = MobiusTransform {
            a: (0.0, 0.0),
            b: (1.0, 0.0),
            c: (1.0, 0.0),
            d: (0.0, 0.0),
        };
        // 1/(2+0i) = 0.5
        let (re, im) = m.apply(2.0, 0.0);
        assert!((re - 0.5).abs() < 1e-12);
        assert!(im.abs() < 1e-12);
    }

    #[test]
    fn test_mobius_compose_with_identity() {
        let m = MobiusTransform {
            a: (2.0, 1.0),
            b: (0.0, 1.0),
            c: (1.0, 0.0),
            d: (1.0, -1.0),
        };
        let id = MobiusTransform::identity();
        let composed = m.compose(&id);
        // Composing with identity should give same result.
        let z = (1.5, -0.5);
        let (re1, im1) = m.apply(z.0, z.1);
        let (re2, im2) = composed.apply(z.0, z.1);
        assert!((re1 - re2).abs() < 1e-10);
        assert!((im1 - im2).abs() < 1e-10);
    }

    // ── Renderer ─────────────────────────────────────────────────────────────

    #[test]
    fn test_render_latitude_buffer_size() {
        let cfg = StereographicConfig {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let buf = StereographicRenderer::render_latitude_lines(&cfg, 3, [0, 0, 255]);
        assert_eq!(buf.len(), 64);
        assert_eq!(buf[0].len(), 64);
    }

    #[test]
    fn test_render_longitude_buffer_size() {
        let cfg = StereographicConfig {
            width: 64,
            height: 64,
            ..Default::default()
        };
        let buf = StereographicRenderer::render_longitude_lines(&cfg, 4, [255, 0, 0]);
        assert_eq!(buf.len(), 64);
        assert_eq!(buf[0].len(), 64);
    }

    #[test]
    fn test_render_circle_inversion_draws_pixels() {
        let cfg = StereographicConfig {
            width: 128,
            height: 128,
            ..Default::default()
        };
        let circles = vec![(0.0f64, 0.0f64, 0.5f64)];
        let buf = StereographicRenderer::render_circle_inversion(&cfg, &circles, [0, 255, 0]);
        // At least one pixel should be non-white.
        let has_color = buf.iter().flatten().any(|&p| p != [255u8; 3]);
        assert!(has_color);
    }

    #[test]
    fn test_mobius_compose_associativity() {
        let m1 = MobiusTransform {
            a: (1.0, 1.0),
            b: (0.0, 1.0),
            c: (1.0, 0.0),
            d: (1.0, 0.0),
        };
        let m2 = MobiusTransform {
            a: (0.5, 0.0),
            b: (1.0, 0.5),
            c: (0.0, 0.5),
            d: (1.0, 0.0),
        };
        let m3 = MobiusTransform::identity();
        let lhs = m1.compose(&m2).compose(&m3);
        let rhs = m1.compose(&m2.compose(&m3));
        let z = (2.0, -1.0);
        let (r1, i1) = lhs.apply(z.0, z.1);
        let (r2, i2) = rhs.apply(z.0, z.1);
        assert!((r1 - r2).abs() < 1e-9);
        assert!((i1 - i2).abs() < 1e-9);
    }
}
