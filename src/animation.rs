//! Animation export: renders a sequence of PNG frames by interpolating parameters over time.
//!
//! This module provides [`AnimationExporter`] which drives parameter interpolation
//! across `N` frames and writes numbered PNG files to an output directory.
//!
//! # Supported animated parameters
//!
//! | `AnimationParameter` | Description |
//! |---------------------|-------------|
//! | `RotationAngle` | Camera orbit angle (radians) |
//! | `Scale` | Overall scene scale |
//! | `ColorHue` | Hue rotation of the color palette (degrees 0..360) |
//! | `WindingNumber` | Symmetry winding parameter (integer-like, interpolated) |
//!
//! # Interpolation modes
//!
//! - `Linear` — evenly spaced between `start` and `end`
//! - `Sinusoidal` — smooth oscillation: `start + (end - start) * 0.5 * (1 - cos(π·t))`
//!
//! # CLI
//!
//! ```text
//! geodesic-wallpaper --animate --frames 60 --fps 30 --out-dir ./frames
//! ```
//!
//! # Example
//!
//! ```no_run
//! use geodesic_wallpaper::animation::{
//!     AnimationConfig, AnimationExporter, AnimationParameter, FrameInterpolator,
//!     InterpolationMode,
//! };
//! use std::path::PathBuf;
//!
//! let config = AnimationConfig {
//!     frames: 60,
//!     fps: 30,
//!     width: 1920,
//!     height: 1080,
//!     output_dir: PathBuf::from("./frames"),
//! };
//! let interp = FrameInterpolator::new(
//!     AnimationParameter::RotationAngle,
//!     0.0,
//!     std::f64::consts::TAU,
//!     InterpolationMode::Linear,
//! );
//! let exporter = AnimationExporter::new(config, vec![interp]);
//! // exporter.export(|values, path| { /* render frame */ Ok(()) }).unwrap();
//! ```

use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

// ── AnimationParameter ────────────────────────────────────────────────────────

/// The parameter to animate over the frame sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationParameter {
    /// Camera orbit angle in radians.
    RotationAngle,
    /// Uniform scene scale factor.
    Scale,
    /// Hue rotation applied to the color palette (degrees, 0..360).
    ColorHue,
    /// Symmetry winding number (integer-like, real-valued during interpolation).
    WindingNumber,
}

// ── Interpolation ─────────────────────────────────────────────────────────────

/// How to interpolate between start and end values over the animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationMode {
    /// Even linear spacing: `value = start + (end - start) * t`.
    Linear,
    /// Smooth cosine easing: `value = start + (end - start) * 0.5 * (1 − cos(π·t))`.
    Sinusoidal,
}

/// Interpolates a single animation parameter across the frame range.
#[derive(Debug, Clone)]
pub struct FrameInterpolator {
    /// The parameter this interpolator controls.
    pub parameter: AnimationParameter,
    /// Value at frame 0.
    pub start: f64,
    /// Value at the last frame.
    pub end: f64,
    /// Interpolation curve.
    pub mode: InterpolationMode,
}

impl FrameInterpolator {
    /// Create a new frame interpolator.
    pub fn new(parameter: AnimationParameter, start: f64, end: f64, mode: InterpolationMode) -> Self {
        Self { parameter, start, end, mode }
    }

    /// Evaluate the parameter at a normalised time `t ∈ [0, 1]`.
    pub fn evaluate(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        let eased = match self.mode {
            InterpolationMode::Linear => t,
            InterpolationMode::Sinusoidal => 0.5 * (1.0 - (std::f64::consts::PI * t).cos()),
        };
        self.start + (self.end - self.start) * eased
    }
}

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for an animation export.
#[derive(Debug, Clone)]
pub struct AnimationConfig {
    /// Number of frames to render.
    pub frames: usize,
    /// Playback frame rate (used for duration calculation only; does not affect PNG output rate).
    pub fps: u32,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Directory to write PNG frames into.
    pub output_dir: PathBuf,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            frames: 60,
            fps: 30,
            width: 1920,
            height: 1080,
            output_dir: PathBuf::from("./frames"),
        }
    }
}

impl AnimationConfig {
    /// Animation duration in seconds.
    pub fn duration_secs(&self) -> f64 {
        self.frames as f64 / self.fps.max(1) as f64
    }
}

// ── Stats ─────────────────────────────────────────────────────────────────────

/// Statistics from a completed animation export.
#[derive(Debug, Clone, Default)]
pub struct AnimationStats {
    /// Number of frames written to disk.
    pub frames_written: usize,
    /// Total wall-clock duration of the export in milliseconds.
    pub duration_ms: u64,
    /// Average time per frame in milliseconds.
    pub avg_frame_ms: f64,
}

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors that can occur during animation export.
#[derive(Debug)]
pub enum ExportError {
    /// An I/O operation failed.
    Io(io::Error),
    /// The caller's render callback returned an error.
    Render(String),
    /// Invalid configuration (e.g. zero frames).
    InvalidConfig(String),
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportError::Io(e) => write!(f, "IO error: {e}"),
            ExportError::Render(s) => write!(f, "render error: {s}"),
            ExportError::InvalidConfig(s) => write!(f, "invalid config: {s}"),
        }
    }
}

impl std::error::Error for ExportError {}

impl From<io::Error> for ExportError {
    fn from(e: io::Error) -> Self {
        ExportError::Io(e)
    }
}

// ── AnimationExporter ─────────────────────────────────────────────────────────

/// Renders a sequence of frames by interpolating parameters and invoking a callback.
///
/// The caller supplies a render function `render_frame(frame_index, param_values, output_path) -> Result<(), String>`.
/// `param_values` maps each [`AnimationParameter`] to its current value.
pub struct AnimationExporter {
    config: AnimationConfig,
    interpolators: Vec<FrameInterpolator>,
}

impl AnimationExporter {
    /// Create a new exporter with the given configuration and interpolators.
    pub fn new(config: AnimationConfig, interpolators: Vec<FrameInterpolator>) -> Self {
        Self { config, interpolators }
    }

    /// Return the path for a given frame index: `{output_dir}/frame_{NNNN}.png`.
    pub fn frame_path(&self, frame_index: usize) -> PathBuf {
        self.config.output_dir.join(format!("frame_{:04}.png", frame_index))
    }

    /// Evaluate all interpolators at the given frame index.
    ///
    /// Returns a `Vec` of `(AnimationParameter, value)` pairs.
    pub fn param_values_at(&self, frame_index: usize) -> Vec<(AnimationParameter, f64)> {
        let t = if self.config.frames <= 1 {
            0.0
        } else {
            frame_index as f64 / (self.config.frames - 1) as f64
        };
        self.interpolators
            .iter()
            .map(|interp| (interp.parameter, interp.evaluate(t)))
            .collect()
    }

    /// Export the animation by calling `render_frame` for each frame.
    ///
    /// `render_frame(frame_index, param_values, output_path)` should write a PNG
    /// to `output_path` and return `Ok(())` on success.
    ///
    /// Frames are named `frame_0000.png` through `frame_NNNN.png`.
    pub fn export<F>(&self, mut render_frame: F) -> Result<AnimationStats, ExportError>
    where
        F: FnMut(usize, &[(AnimationParameter, f64)], &Path) -> Result<(), String>,
    {
        if self.config.frames == 0 {
            return Err(ExportError::InvalidConfig("frames must be > 0".to_string()));
        }

        // Ensure output directory exists
        std::fs::create_dir_all(&self.config.output_dir)?;

        let start = Instant::now();
        let mut frames_written = 0usize;

        for frame_idx in 0..self.config.frames {
            let params = self.param_values_at(frame_idx);
            let output_path = self.frame_path(frame_idx);

            render_frame(frame_idx, &params, &output_path)
                .map_err(ExportError::Render)?;

            frames_written += 1;
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let avg_frame_ms = if frames_written > 0 {
            duration_ms as f64 / frames_written as f64
        } else {
            0.0
        };

        Ok(AnimationStats {
            frames_written,
            duration_ms,
            avg_frame_ms,
        })
    }

    /// Write a minimal valid 1x1 white PNG to the given path (useful for testing without a GPU).
    ///
    /// This is a stdlib-only PNG encoder — it writes a valid PNG with no external dependencies.
    pub fn write_test_frame(path: &Path, width: u32, height: u32) -> Result<(), ExportError> {
        use std::io::Write;

        let mut pixels = vec![255u8; (width * height * 4) as usize];
        // Fill a simple gradient for visual interest
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                pixels[idx] = (x * 255 / width.max(1)) as u8;     // R
                pixels[idx + 1] = (y * 255 / height.max(1)) as u8; // G
                pixels[idx + 2] = 128;                              // B
                pixels[idx + 3] = 255;                              // A
            }
        }

        encode_png(path, width, height, &pixels)
            .map_err(ExportError::Io)
    }
}

/// Minimal PNG encoder using stdlib only (no external crate).
///
/// Writes an RGBA PNG file. Uses Zlib level-0 compression (stored, no compression)
/// to keep the implementation simple and dependency-free.
fn encode_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> io::Result<()> {
    use std::io::Write;

    let mut f = std::fs::File::create(path)?;

    // PNG signature
    f.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;

    // IHDR chunk
    let ihdr_data: Vec<u8> = {
        let mut d = Vec::with_capacity(13);
        d.extend_from_slice(&width.to_be_bytes());
        d.extend_from_slice(&height.to_be_bytes());
        d.push(8);  // bit depth
        d.push(6);  // RGBA
        d.push(0);  // compression method
        d.push(0);  // filter method
        d.push(0);  // interlace
        d
    };
    write_png_chunk(&mut f, b"IHDR", &ihdr_data)?;

    // Build raw scanlines with filter byte 0 (None)
    let row_len = (width * 4) as usize;
    let mut raw = Vec::with_capacity((row_len + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0u8); // filter byte
        raw.extend_from_slice(&rgba[row * row_len..(row + 1) * row_len]);
    }

    // Wrap in a Deflate "stored" block (no compression, max DEFLATE block = 65535 bytes)
    let compressed = deflate_store(&raw);

    // IDAT
    write_png_chunk(&mut f, b"IDAT", &compressed)?;

    // IEND
    write_png_chunk(&mut f, b"IEND", &[])?;

    Ok(())
}

fn write_png_chunk(f: &mut impl std::io::Write, tag: &[u8; 4], data: &[u8]) -> io::Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    f.write_all(&len)?;
    f.write_all(tag)?;
    f.write_all(data)?;
    let crc = png_crc(tag, data);
    f.write_all(&crc.to_be_bytes())?;
    Ok(())
}

/// CRC-32 used by PNG.
fn png_crc(tag: &[u8], data: &[u8]) -> u32 {
    let table = crc32_table();
    let mut crc = 0xFFFF_FFFFu32;
    for &b in tag.iter().chain(data.iter()) {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

fn crc32_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for n in 0u32..256 {
        let mut c = n;
        for _ in 0..8 {
            if c & 1 != 0 {
                c = 0xEDB8_8320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
        }
        table[n as usize] = c;
    }
    table
}

/// Wrap raw bytes in a DEFLATE "stored" (uncompressed) stream with Zlib header/trailer.
fn deflate_store(data: &[u8]) -> Vec<u8> {
    const BLOCK_MAX: usize = 65535;
    let mut out = Vec::new();

    // Zlib header: CMF=0x78, FLG computed so CMF*256+FLG is divisible by 31
    // 0x78 = deflate + window 32KiB; 0x01 makes the check pass (0x7801 % 31 == 0)
    out.push(0x78);
    out.push(0x01);

    let chunks: Vec<&[u8]> = data.chunks(BLOCK_MAX).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        out.push(if is_last { 1 } else { 0 }); // BFINAL, BTYPE=00 (stored)
        let len = chunk.len() as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(chunk);
    }
    if data.is_empty() {
        out.push(1); // BFINAL=1, BTYPE=00
        out.extend_from_slice(&[0x00, 0x00, 0xFF, 0xFF]); // LEN=0, NLEN=~0
    }

    // Adler-32 checksum
    let (s1, s2) = data.iter().fold((1u32, 0u32), |(s1, s2), &b| {
        let s1 = (s1 + b as u32) % 65521;
        let s2 = (s2 + s1) % 65521;
        (s1, s2)
    });
    let adler = (s2 << 16) | s1;
    out.extend_from_slice(&adler.to_be_bytes());

    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_interpolation_endpoints() {
        let interp = FrameInterpolator::new(AnimationParameter::RotationAngle, 0.0, 1.0, InterpolationMode::Linear);
        assert!((interp.evaluate(0.0) - 0.0).abs() < 1e-12);
        assert!((interp.evaluate(1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_linear_interpolation_midpoint() {
        let interp = FrameInterpolator::new(AnimationParameter::Scale, 0.0, 2.0, InterpolationMode::Linear);
        assert!((interp.evaluate(0.5) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_sinusoidal_interpolation_endpoints() {
        let interp = FrameInterpolator::new(AnimationParameter::ColorHue, 0.0, 360.0, InterpolationMode::Sinusoidal);
        assert!((interp.evaluate(0.0) - 0.0).abs() < 1e-9);
        assert!((interp.evaluate(1.0) - 360.0).abs() < 1e-9);
    }

    #[test]
    fn test_sinusoidal_interpolation_midpoint() {
        // At t=0.5: eased = 0.5*(1-cos(π*0.5)) = 0.5*(1-0) = 0.5
        let interp = FrameInterpolator::new(AnimationParameter::Scale, 0.0, 1.0, InterpolationMode::Sinusoidal);
        assert!((interp.evaluate(0.5) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_interpolation_clamps_t() {
        let interp = FrameInterpolator::new(AnimationParameter::Scale, 1.0, 2.0, InterpolationMode::Linear);
        assert!((interp.evaluate(-1.0) - 1.0).abs() < 1e-12);
        assert!((interp.evaluate(2.0) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn test_animation_config_duration() {
        let cfg = AnimationConfig { frames: 60, fps: 30, ..AnimationConfig::default() };
        assert!((cfg.duration_secs() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn test_animation_config_duration_zero_fps() {
        let cfg = AnimationConfig { frames: 30, fps: 0, ..AnimationConfig::default() };
        // fps clamped to 1 → duration = 30s
        assert!((cfg.duration_secs() - 30.0).abs() < 1e-12);
    }

    #[test]
    fn test_frame_path_format() {
        let exporter = AnimationExporter::new(
            AnimationConfig { output_dir: PathBuf::from("/tmp/test"), ..AnimationConfig::default() },
            vec![],
        );
        let p = exporter.frame_path(42);
        assert!(p.to_string_lossy().contains("frame_0042.png"));
    }

    #[test]
    fn test_param_values_at_first_frame() {
        let interp = FrameInterpolator::new(AnimationParameter::RotationAngle, 1.0, 5.0, InterpolationMode::Linear);
        let exporter = AnimationExporter::new(
            AnimationConfig { frames: 10, ..AnimationConfig::default() },
            vec![interp],
        );
        let vals = exporter.param_values_at(0);
        assert_eq!(vals.len(), 1);
        assert!((vals[0].1 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_param_values_at_last_frame() {
        let interp = FrameInterpolator::new(AnimationParameter::RotationAngle, 1.0, 5.0, InterpolationMode::Linear);
        let exporter = AnimationExporter::new(
            AnimationConfig { frames: 10, ..AnimationConfig::default() },
            vec![interp],
        );
        let vals = exporter.param_values_at(9);
        assert!((vals[0].1 - 5.0).abs() < 1e-12);
    }

    #[test]
    fn test_export_zero_frames_error() {
        let exporter = AnimationExporter::new(
            AnimationConfig { frames: 0, ..AnimationConfig::default() },
            vec![],
        );
        let result = exporter.export(|_, _, _| Ok(()));
        assert!(matches!(result, Err(ExportError::InvalidConfig(_))));
    }

    #[test]
    fn test_export_writes_correct_frame_count() {
        let dir = tempfile::tempdir().unwrap();
        let exporter = AnimationExporter::new(
            AnimationConfig { frames: 5, fps: 10, width: 4, height: 4, output_dir: dir.path().to_path_buf() },
            vec![],
        );
        let stats = exporter.export(|_, _, _| { Ok(()) }).unwrap();
        assert_eq!(stats.frames_written, 5);
    }

    #[test]
    fn test_export_propagates_render_error() {
        let dir = tempfile::tempdir().unwrap();
        let exporter = AnimationExporter::new(
            AnimationConfig { frames: 3, ..AnimationConfig::default().with_dir(dir.path()) },
            vec![],
        );
        let result = exporter.export(|i, _, _| {
            if i == 1 { Err("render failed at frame 1".to_string()) } else { Ok(()) }
        });
        assert!(matches!(result, Err(ExportError::Render(_))));
    }

    #[test]
    fn test_write_test_frame_produces_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_frame.png");
        AnimationExporter::write_test_frame(&path, 4, 4).unwrap();
        assert!(path.exists());
        let meta = std::fs::metadata(&path).unwrap();
        assert!(meta.len() > 8, "PNG file should have content");
    }

    #[test]
    fn test_write_test_frame_valid_png_signature() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sig_test.png");
        AnimationExporter::write_test_frame(&path, 2, 2).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10], "PNG signature mismatch");
    }

    #[test]
    fn test_animation_stats_default() {
        let stats = AnimationStats::default();
        assert_eq!(stats.frames_written, 0);
        assert_eq!(stats.duration_ms, 0);
        assert_eq!(stats.avg_frame_ms, 0.0);
    }

    #[test]
    fn test_multiple_interpolators() {
        let interps = vec![
            FrameInterpolator::new(AnimationParameter::RotationAngle, 0.0, 6.28, InterpolationMode::Linear),
            FrameInterpolator::new(AnimationParameter::ColorHue, 0.0, 360.0, InterpolationMode::Sinusoidal),
        ];
        let exporter = AnimationExporter::new(
            AnimationConfig { frames: 4, ..AnimationConfig::default() },
            interps,
        );
        let vals = exporter.param_values_at(2);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].0, AnimationParameter::RotationAngle);
        assert_eq!(vals[1].0, AnimationParameter::ColorHue);
    }
}

// ── Helper extension trait for testing ───────────────────────────────────────

impl AnimationConfig {
    fn with_dir(mut self, dir: &Path) -> Self {
        self.output_dir = dir.to_path_buf();
        self
    }
}
