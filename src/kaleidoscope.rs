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
