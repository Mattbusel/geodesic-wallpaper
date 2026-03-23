//! Kaleidoscope symmetry renderer.
//!
//! Folds a source image into a fundamental domain and tiles it with rotational
//! symmetry to produce a kaleidoscope effect.  All operations are pure
//! float-point geometry with no external dependencies.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Configuration for a kaleidoscope render pass.
#[derive(Debug, Clone)]
pub struct KaleidoscopeConfig {
    /// Number of mirror segments (must be ≥ 2).
    pub segments: u32,
    /// Output image width in pixels.
    pub width: u32,
    /// Output image height in pixels.
    pub height: u32,
    /// Global rotation offset in radians.
    pub rotation: f64,
}

impl KaleidoscopeConfig {
    pub fn new(segments: u32, width: u32, height: u32, rotation: f64) -> Self {
        assert!(segments >= 2, "segments must be at least 2");
        KaleidoscopeConfig {
            segments,
            width,
            height,
            rotation,
        }
    }
}

// ---------------------------------------------------------------------------
// Geometric helpers
// ---------------------------------------------------------------------------

/// Reflect point `(x, y)` across a line through the origin at `angle` radians.
pub fn reflect_point(x: f64, y: f64, angle: f64) -> (f64, f64) {
    // Reflection formula across line y = tan(angle) * x.
    let cos2 = (2.0 * angle).cos();
    let sin2 = (2.0 * angle).sin();
    let rx = cos2 * x + sin2 * y;
    let ry = sin2 * x - cos2 * y;
    (rx, ry)
}

/// Fold `(x, y)` into the fundamental domain of the kaleidoscope.
///
/// The fundamental domain is the sector [0, π/segments).
/// Points are repeatedly reflected until they land inside this sector.
pub fn kaleidoscope_transform(x: f64, y: f64, config: &KaleidoscopeConfig) -> (f64, f64) {
    let sector_angle = PI / config.segments as f64;

    // Convert to polar.
    let r = (x * x + y * y).sqrt();
    let mut theta = y.atan2(x) - config.rotation;

    // Wrap theta into [0, 2π).
    theta = theta.rem_euclid(2.0 * PI);

    // Fold into fundamental sector using modular reflection.
    // Each full rotation has `2 * segments` half-sectors.
    let full_sectors = (2 * config.segments) as f64;
    let sector_idx = (theta / sector_angle).floor() as u32;

    // Mirror on every odd sector.
    if sector_idx % 2 == 1 {
        theta = sector_angle * (sector_idx as f64 + 1.0) - (theta - sector_angle * sector_idx as f64);
    } else {
        theta -= sector_angle * sector_idx as f64;
    }

    // Clamp to fundamental domain.
    theta = theta.clamp(0.0, sector_angle);

    let _ = full_sectors; // suppress lint

    // Back to Cartesian.
    (r * theta.cos(), r * theta.sin())
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// Render a kaleidoscope image from `source`.
///
/// `source` is a 2-D grid of RGB pixels `[row][col]`.  For each output pixel
/// the kaleidoscope transform is applied and the resulting coordinate is
/// sampled from the source with nearest-neighbour lookup.
pub fn render_kaleidoscope(
    source: &Vec<Vec<[u8; 3]>>,
    config: &KaleidoscopeConfig,
) -> Vec<Vec<[u8; 3]>> {
    let out_h = config.height as usize;
    let out_w = config.width as usize;
    let src_h = source.len().max(1);
    let src_w = source.first().map(|r| r.len()).unwrap_or(1).max(1);

    let cx = out_w as f64 / 2.0;
    let cy = out_h as f64 / 2.0;

    let mut output = vec![vec![[0u8; 3]; out_w]; out_h];

    for row in 0..out_h {
        for col in 0..out_w {
            // Normalise to centred coordinates.
            let x = (col as f64 - cx) / cx;
            let y = (row as f64 - cy) / cy;

            let (tx, ty) = kaleidoscope_transform(x, y, config);

            // Map transformed coords back into source image space.
            let src_col = ((tx * cx + cx) as usize).min(src_w - 1);
            let src_row = ((ty * cy + cy) as usize).min(src_h - 1);

            output[row][col] = source[src_row][src_col];
        }
    }

    output
}

// ---------------------------------------------------------------------------
// KaleidoscopeAnimator
// ---------------------------------------------------------------------------

/// Generates animation frames by stepping the rotation angle each frame.
pub struct KaleidoscopeAnimator {
    config: KaleidoscopeConfig,
}

impl KaleidoscopeAnimator {
    pub fn new(config: KaleidoscopeConfig) -> Self {
        KaleidoscopeAnimator { config }
    }

    /// Advance the rotation by `delta_radians` and return the new frame.
    pub fn step(&mut self, source: &Vec<Vec<[u8; 3]>>, delta_radians: f64) -> Vec<Vec<[u8; 3]>> {
        self.config.rotation += delta_radians;
        render_kaleidoscope(source, &self.config)
    }

    /// Generate `n` animation frames, each rotated by `delta_radians`.
    pub fn generate_frames(
        &mut self,
        source: &Vec<Vec<[u8; 3]>>,
        n: usize,
        delta_radians: f64,
    ) -> Vec<Vec<Vec<[u8; 3]>>> {
        (0..n).map(|_| self.step(source, delta_radians)).collect()
    }

    pub fn current_rotation(&self) -> f64 {
        self.config.rotation
    }
}

// ---------------------------------------------------------------------------
// Round 27: KaleidoscopeRenderer with extended config
// ---------------------------------------------------------------------------

/// Extended configuration for the new renderer API.
#[derive(Debug, Clone)]
pub struct KaleidoscopeConfig2 {
    pub n_fold: u32,
    pub rotation_offset: f64,
    pub zoom: f64,
    pub center: (f64, f64),
    pub color_rotation: f64,
}

impl Default for KaleidoscopeConfig2 {
    fn default() -> Self {
        KaleidoscopeConfig2 {
            n_fold: 6,
            rotation_offset: 0.0,
            zoom: 1.0,
            center: (0.0, 0.0),
            color_rotation: 0.0,
        }
    }
}

/// Kaleidoscope renderer using the extended config.
pub struct KaleidoscopeRenderer;

impl KaleidoscopeRenderer {
    /// Map any point `(x, y)` to the fundamental domain via angle folding.
    pub fn map_to_sector(x: f64, y: f64, config: &KaleidoscopeConfig2) -> (f64, f64) {
        let n = config.n_fold.max(1) as f64;
        let sector_angle = PI / n;

        // Translate by center
        let cx = x - config.center.0;
        let cy = y - config.center.1;

        let r = (cx * cx + cy * cy).sqrt() / config.zoom.max(0.001);
        let mut theta = cy.atan2(cx) - config.rotation_offset;
        theta = theta.rem_euclid(2.0 * PI);

        // Fold into [0, sector_angle]
        let sector_idx = (theta / sector_angle).floor() as u32;
        if sector_idx % 2 == 1 {
            theta = sector_angle * (sector_idx as f64 + 1.0)
                - (theta - sector_angle * sector_idx as f64);
        } else {
            theta -= sector_angle * sector_idx as f64;
        }
        theta = theta.clamp(0.0, sector_angle);

        (r * theta.cos(), r * theta.sin())
    }

    /// Render source image with kaleidoscope folding.
    pub fn render(
        source: &Vec<Vec<[u8; 3]>>,
        config: &KaleidoscopeConfig2,
        width: u32,
        height: u32,
    ) -> Vec<Vec<[u8; 3]>> {
        let w = width as usize;
        let h = height as usize;
        let src_h = source.len().max(1);
        let src_w = source.first().map(|r| r.len()).unwrap_or(1).max(1);
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        let mut out = vec![vec![[0u8; 3]; w]; h];

        for row in 0..h {
            for col in 0..w {
                let x = (col as f64 - cx) / cx;
                let y = (row as f64 - cy) / cy;
                let (tx, ty) = Self::map_to_sector(x, y, config);
                let sc = Self::sample_bilinear(source, tx * cx + cx, ty * cy + cy);
                let col_out = Self::rotate_color(sc, config.color_rotation);
                out[row][col] = col_out;
                let _ = (src_h, src_w);
            }
        }
        out
    }

    /// Render a kaleidoscope from a procedural function.
    pub fn render_pattern(
        f: impl Fn(f64, f64) -> [u8; 3],
        config: &KaleidoscopeConfig2,
        width: u32,
        height: u32,
    ) -> Vec<Vec<[u8; 3]>> {
        let w = width as usize;
        let h = height as usize;
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;

        let mut out = vec![vec![[0u8; 3]; w]; h];
        for row in 0..h {
            for col in 0..w {
                let x = (col as f64 - cx) / cx;
                let y = (row as f64 - cy) / cy;
                let (tx, ty) = Self::map_to_sector(x, y, config);
                let color = f(tx, ty);
                out[row][col] = Self::rotate_color(color, config.color_rotation);
            }
        }
        out
    }

    /// Bilinear sample from source at floating-point pixel coords.
    pub fn sample_bilinear(source: &Vec<Vec<[u8; 3]>>, x: f64, y: f64) -> [u8; 3] {
        let h = source.len();
        if h == 0 { return [0, 0, 0]; }
        let w = source[0].len();
        if w == 0 { return [0, 0, 0]; }

        let x = x.clamp(0.0, (w - 1) as f64);
        let y = y.clamp(0.0, (h - 1) as f64);

        let i0 = (x as usize).min(w - 1);
        let j0 = (y as usize).min(h - 1);
        let i1 = (i0 + 1).min(w - 1);
        let j1 = (j0 + 1).min(h - 1);
        let sf = x - i0 as f64;
        let tf = y - j0 as f64;

        let lerp = |a: u8, b: u8, t: f64| -> u8 {
            (a as f64 * (1.0 - t) + b as f64 * t).round().clamp(0.0, 255.0) as u8
        };

        let top    = [lerp(source[j0][i0][0], source[j0][i1][0], sf),
                      lerp(source[j0][i0][1], source[j0][i1][1], sf),
                      lerp(source[j0][i0][2], source[j0][i1][2], sf)];
        let bottom = [lerp(source[j1][i0][0], source[j1][i1][0], sf),
                      lerp(source[j1][i0][1], source[j1][i1][1], sf),
                      lerp(source[j1][i0][2], source[j1][i1][2], sf)];
        [lerp(top[0], bottom[0], tf),
         lerp(top[1], bottom[1], tf),
         lerp(top[2], bottom[2], tf)]
    }

    /// Rotate hue of `color` by `angle` (in [0, 1] turns).
    pub fn rotate_color(color: [u8; 3], angle: f64) -> [u8; 3] {
        let (h, s, v) = Self::rgb_to_hsv(color[0], color[1], color[2]);
        let new_h = (h + angle).rem_euclid(1.0);
        Self::hsv_to_rgb(new_h, s, v)
    }

    /// Convert HSV (each in [0, 1]) to RGB `[u8; 3]`.
    pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
        let h6 = h.rem_euclid(1.0) * 6.0;
        let i = h6.floor() as u32;
        let f = h6 - h6.floor();
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

    /// Convert RGB `u8` components to HSV (each in [0, 1]).
    pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
        let r = r as f64 / 255.0;
        let g = g as f64 / 255.0;
        let b = b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max < 1e-9 { 0.0 } else { delta / max };
        let h = if delta < 1e-9 {
            0.0
        } else if (max - r).abs() < 1e-9 {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if (max - g).abs() < 1e-9 {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };
        (h, s, v)
    }

    /// Advance rotation and return a modified config for animation.
    pub fn animated_frame(config: &mut KaleidoscopeConfig2, frame: u32, fps: f64) -> KaleidoscopeConfig2 {
        let t = frame as f64 / fps.max(1.0);
        let mut c = config.clone();
        c.rotation_offset = config.rotation_offset + t * 0.1;
        c.color_rotation = config.color_rotation + t * 0.02;
        c
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_source(h: usize, w: usize, color: [u8; 3]) -> Vec<Vec<[u8; 3]>> {
        vec![vec![color; w]; h]
    }

    fn checkerboard(h: usize, w: usize) -> Vec<Vec<[u8; 3]>> {
        (0..h)
            .map(|r| {
                (0..w)
                    .map(|c| {
                        if (r + c) % 2 == 0 {
                            [255, 255, 255]
                        } else {
                            [0, 0, 0]
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn reflect_point_across_x_axis() {
        // Reflecting (1, 1) across angle 0 (x-axis) should give (1, -1).
        let (rx, ry) = reflect_point(1.0, 1.0, 0.0);
        assert!((rx - 1.0).abs() < 1e-9, "rx={}", rx);
        assert!((ry + 1.0).abs() < 1e-9, "ry={}", ry);
    }

    #[test]
    fn reflect_point_across_45_degrees() {
        // Reflecting (1, 0) across 45° should give (0, 1).
        let (rx, ry) = reflect_point(1.0, 0.0, PI / 4.0);
        assert!((rx - 0.0).abs() < 1e-9, "rx={}", rx);
        assert!((ry - 1.0).abs() < 1e-9, "ry={}", ry);
    }

    #[test]
    fn kaleidoscope_transform_stays_in_sector() {
        let config = KaleidoscopeConfig::new(6, 100, 100, 0.0);
        let sector_angle = PI / 6.0;
        for &(x, y) in &[(0.5, 0.3), (-0.2, 0.7), (0.0, 0.5), (-0.4, -0.4)] {
            let (tx, ty) = kaleidoscope_transform(x, y, &config);
            let theta = ty.atan2(tx);
            assert!(
                theta >= -1e-6 && theta <= sector_angle + 1e-6,
                "theta={} out of [0, sector_angle] for ({}, {})",
                theta,
                x,
                y
            );
        }
    }

    #[test]
    fn render_kaleidoscope_output_dimensions() {
        let source = solid_source(64, 64, [128, 0, 200]);
        let config = KaleidoscopeConfig::new(8, 32, 32, 0.0);
        let output = render_kaleidoscope(&source, &config);
        assert_eq!(output.len(), 32);
        assert_eq!(output[0].len(), 32);
    }

    #[test]
    fn render_kaleidoscope_solid_source_preserves_color() {
        let color = [100u8, 150, 200];
        let source = solid_source(64, 64, color);
        let config = KaleidoscopeConfig::new(6, 32, 32, 0.0);
        let output = render_kaleidoscope(&source, &config);
        // Every output pixel should be the same solid color.
        for row in &output {
            for &px in row {
                assert_eq!(px, color);
            }
        }
    }

    #[test]
    fn render_kaleidoscope_checkerboard_runs() {
        let source = checkerboard(64, 64);
        let config = KaleidoscopeConfig::new(4, 32, 32, 0.0);
        let output = render_kaleidoscope(&source, &config);
        assert_eq!(output.len(), 32);
    }

    #[test]
    fn animator_generates_n_frames() {
        let source = solid_source(32, 32, [255, 0, 0]);
        let config = KaleidoscopeConfig::new(4, 16, 16, 0.0);
        let mut animator = KaleidoscopeAnimator::new(config);
        let frames = animator.generate_frames(&source, 5, 0.1);
        assert_eq!(frames.len(), 5);
    }

    #[test]
    fn animator_rotation_advances() {
        let source = solid_source(32, 32, [0, 255, 0]);
        let config = KaleidoscopeConfig::new(4, 16, 16, 0.0);
        let mut animator = KaleidoscopeAnimator::new(config);
        animator.step(&source, 0.5);
        assert!((animator.current_rotation() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn config_panics_on_zero_segments() {
        let result = std::panic::catch_unwind(|| {
            KaleidoscopeConfig::new(1, 100, 100, 0.0);
        });
        assert!(result.is_err());
    }
}
