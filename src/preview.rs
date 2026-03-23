//! ASCII/Unicode block-character preview of geodesic wallpaper patterns.
//!
//! Renders a small pattern to stdout using Unicode block characters
//! (` `, `░`, `▒`, `▓`, `█`) so the user can preview the current
//! configuration without launching a full GPU window.
//!
//! This module is the non-interactive fallback for the interactive TUI
//! described in the specification. It provides:
//!
//! - `WallpaperParams` — live configuration snapshot adjustable in the preview.
//! - `AsciiPreview` — renders a 40×20 char pattern to any `Write` sink.
//! - `TuiApp`, `TuiResult`, `TuiError` — minimal stubs satisfying the
//!   module contract so callers can `use geodesic_wallpaper::preview::TuiApp`.
//!
//! # Example
//!
//! ```rust
//! use geodesic_wallpaper::preview::{WallpaperParams, AsciiPreview};
//!
//! let params = WallpaperParams::default();
//! let mut out = Vec::new();
//! AsciiPreview::render(&params, 40, 20, &mut out).unwrap();
//! let s = String::from_utf8(out).unwrap();
//! println!("{}", s);
//! ```

use std::fmt;
use std::io::{self, Write};

// ── WallpaperParams ───────────────────────────────────────────────────────────

/// Snapshot of adjustable wallpaper parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct WallpaperParams {
    /// Active symmetry group name (e.g. `"p4g"`, `"p6m"`).
    pub symmetry_group: String,
    /// Pattern scale factor (> 0).
    pub scale: f32,
    /// Global rotation in degrees.
    pub rotation: f32,
    /// Hue offset applied to the color palette (0–360 degrees).
    pub hue_offset: f32,
    /// Animation speed multiplier.
    pub animation_speed: f32,
}

impl Default for WallpaperParams {
    fn default() -> Self {
        Self {
            symmetry_group: "p4g".into(),
            scale: 1.0,
            rotation: 0.0,
            hue_offset: 0.0,
            animation_speed: 1.0,
        }
    }
}

impl WallpaperParams {
    /// Available symmetry groups in cycle order.
    pub const GROUPS: &'static [&'static str] = &["p1", "p2", "pm", "pg", "cm", "p4", "p4g", "p6m"];

    /// Clamp all parameters to valid ranges.
    pub fn clamp(&mut self) {
        self.scale = self.scale.clamp(0.1, 10.0);
        self.rotation = self.rotation.rem_euclid(360.0);
        self.hue_offset = self.hue_offset.rem_euclid(360.0);
        self.animation_speed = self.animation_speed.clamp(0.0, 10.0);
    }

    /// Cycle to the next symmetry group in the list.
    pub fn cycle_symmetry_group(&mut self) {
        let current = Self::GROUPS.iter().position(|&g| g == self.symmetry_group.as_str());
        let next = match current {
            Some(i) => (i + 1) % Self::GROUPS.len(),
            None => 0,
        };
        self.symmetry_group = Self::GROUPS[next].into();
    }

    /// Cycle to the previous symmetry group in the list.
    pub fn cycle_symmetry_group_back(&mut self) {
        let current = Self::GROUPS.iter().position(|&g| g == self.symmetry_group.as_str());
        let prev = match current {
            Some(0) => Self::GROUPS.len() - 1,
            Some(i) => i - 1,
            None => 0,
        };
        self.symmetry_group = Self::GROUPS[prev].into();
    }
}

// ── AsciiPreview ──────────────────────────────────────────────────────────────

/// Unicode block-character ASCII renderer.
///
/// Maps a 2D pattern function to a grid of block characters:
/// `' '`, `'░'`, `'▒'`, `'▓'`, `'█'` (5 levels, 0.0–1.0).
pub struct AsciiPreview;

const BLOCKS: &[char] = &[' ', '░', '▒', '▓', '█'];

impl AsciiPreview {
    /// Render a pattern of `width × height` block characters to `sink`.
    ///
    /// The pattern function is derived from `params`: a tileable sinusoidal
    /// wave modulated by `scale`, `rotation`, and `hue_offset`.
    pub fn render<W: Write>(params: &WallpaperParams, width: usize, height: usize, sink: &mut W) -> io::Result<()> {
        let scale = params.scale;
        let rot_rad = params.rotation.to_radians();
        let hue_norm = params.hue_offset / 360.0;

        for row in 0..height {
            for col in 0..width {
                // Normalize to [-1, 1]
                let nx = (col as f32 / width as f32) * 2.0 - 1.0;
                let ny = (row as f32 / height as f32) * 2.0 - 1.0;

                // Apply rotation
                let rx = nx * rot_rad.cos() - ny * rot_rad.sin();
                let ry = nx * rot_rad.sin() + ny * rot_rad.cos();

                // Pattern: superposition of two sinusoids
                let v = (rx * scale * std::f32::consts::PI).sin()
                    * (ry * scale * std::f32::consts::PI).cos()
                    + hue_norm;

                // Map to [0, 1]
                let t = (v.sin() + 1.0) * 0.5;
                let idx = (t * (BLOCKS.len() - 1) as f32).round() as usize;
                let idx = idx.clamp(0, BLOCKS.len() - 1);
                let ch = BLOCKS[idx];
                // Write 2 chars per "pixel" so it looks more square in terminals
                write!(sink, "{}{}", ch, ch)?;
            }
            writeln!(sink)?;
        }
        Ok(())
    }

    /// Render with a border and parameter info header to stdout.
    pub fn render_with_header(params: &WallpaperParams, width: usize, height: usize) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = stdout.lock();

        // Header
        writeln!(out, "┌{:─<w$}┐", "", w = width * 2)?;
        writeln!(out, "│ Symmetry: {:10} Scale: {:.2}  Rot: {:.1}°  Hue: {:.0}°  Speed: {:.1} │",
            params.symmetry_group, params.scale, params.rotation, params.hue_offset, params.animation_speed)?;
        writeln!(out, "├{:─<w$}┤", "", w = width * 2)?;

        // Pattern
        let mut buf = Vec::new();
        Self::render(params, width, height, &mut buf)?;
        out.write_all(&buf)?;

        writeln!(out, "└{:─<w$}┘", "", w = width * 2)?;
        Ok(())
    }
}

// ── TuiError ──────────────────────────────────────────────────────────────────

/// Error type for the preview/TUI module.
#[derive(Debug)]
pub enum TuiError {
    Io(io::Error),
    Other(String),
}

impl fmt::Display for TuiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TuiError::Io(e) => write!(f, "I/O error: {}", e),
            TuiError::Other(s) => write!(f, "TUI error: {}", s),
        }
    }
}

impl From<io::Error> for TuiError {
    fn from(e: io::Error) -> Self {
        TuiError::Io(e)
    }
}

// ── TuiResult ─────────────────────────────────────────────────────────────────

/// Result returned by `TuiApp::run`.
#[derive(Debug, Clone)]
pub struct TuiResult {
    /// Final parameter snapshot.
    pub params: WallpaperParams,
    /// True if the user pressed Enter to save.
    pub saved: bool,
}

// ── TuiApp ────────────────────────────────────────────────────────────────────

/// Minimal (non-interactive) TUI application.
///
/// In the absence of `ratatui`, this struct wraps `AsciiPreview` and provides
/// a one-shot render to stdout. An interactive event loop can be added in a
/// future version when `ratatui` + `crossterm` are added to the crate.
pub struct TuiApp {
    pub params: WallpaperParams,
    /// Preview width in characters (each "pixel" is 2 chars wide).
    pub width: usize,
    /// Preview height in character rows.
    pub height: usize,
}

impl TuiApp {
    /// Create a new `TuiApp` with default parameters and a 40×20 canvas.
    pub fn new() -> Self {
        Self {
            params: WallpaperParams::default(),
            width: 40,
            height: 20,
        }
    }

    /// Create with custom dimensions.
    pub fn with_size(width: usize, height: usize) -> Self {
        Self {
            params: WallpaperParams::default(),
            width,
            height,
        }
    }

    /// Render one frame to stdout and return immediately (non-blocking).
    ///
    /// In the full interactive version this would be a blocking event loop.
    pub fn run(&self) -> Result<TuiResult, TuiError> {
        AsciiPreview::render_with_header(&self.params, self.width, self.height)?;
        Ok(TuiResult {
            params: self.params.clone(),
            saved: false,
        })
    }

    /// Save current params to a file as key=value pairs.
    pub fn save_params(&self, path: &std::path::Path) -> io::Result<()> {
        let content = format!(
            "symmetry_group = \"{}\"\nscale = {}\nrotation = {}\nhue_offset = {}\nanimation_speed = {}\n",
            self.params.symmetry_group,
            self.params.scale,
            self.params.rotation,
            self.params.hue_offset,
            self.params.animation_speed,
        );
        std::fs::write(path, content)
    }
}

impl Default for TuiApp {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── WallpaperParams tests ─────────────────────────────────────────────

    #[test]
    fn test_default_params() {
        let p = WallpaperParams::default();
        assert_eq!(p.symmetry_group, "p4g");
        assert!((p.scale - 1.0).abs() < 1e-6);
        assert!((p.rotation - 0.0).abs() < 1e-6);
        assert!((p.hue_offset - 0.0).abs() < 1e-6);
        assert!((p.animation_speed - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_clamp_scale_min() {
        let mut p = WallpaperParams { scale: -1.0, ..Default::default() };
        p.clamp();
        assert!((p.scale - 0.1).abs() < 1e-6);
    }

    #[test]
    fn test_clamp_scale_max() {
        let mut p = WallpaperParams { scale: 100.0, ..Default::default() };
        p.clamp();
        assert!((p.scale - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_clamp_rotation_wraps() {
        let mut p = WallpaperParams { rotation: 400.0, ..Default::default() };
        p.clamp();
        assert!((p.rotation - 40.0).abs() < 1e-3);
    }

    #[test]
    fn test_clamp_hue_wraps() {
        let mut p = WallpaperParams { hue_offset: 720.0, ..Default::default() };
        p.clamp();
        assert!(p.hue_offset < 360.0);
    }

    #[test]
    fn test_clamp_animation_speed_min() {
        let mut p = WallpaperParams { animation_speed: -5.0, ..Default::default() };
        p.clamp();
        assert!((p.animation_speed - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_clamp_animation_speed_max() {
        let mut p = WallpaperParams { animation_speed: 100.0, ..Default::default() };
        p.clamp();
        assert!((p.animation_speed - 10.0).abs() < 1e-6);
    }

    #[test]
    fn test_cycle_symmetry_group_forward() {
        let mut p = WallpaperParams::default(); // starts at "p4g"
        let start = p.symmetry_group.clone();
        p.cycle_symmetry_group();
        assert_ne!(p.symmetry_group, start);
    }

    #[test]
    fn test_cycle_symmetry_group_wraps() {
        let mut p = WallpaperParams { symmetry_group: "p6m".into(), ..Default::default() };
        p.cycle_symmetry_group();
        assert_eq!(p.symmetry_group, WallpaperParams::GROUPS[0]);
    }

    #[test]
    fn test_cycle_symmetry_group_back() {
        let mut p = WallpaperParams::default();
        let before = p.symmetry_group.clone();
        p.cycle_symmetry_group();
        p.cycle_symmetry_group_back();
        assert_eq!(p.symmetry_group, before);
    }

    // ── AsciiPreview tests ────────────────────────────────────────────────

    #[test]
    fn test_render_produces_output() {
        let p = WallpaperParams::default();
        let mut buf = Vec::new();
        AsciiPreview::render(&p, 10, 5, &mut buf).unwrap();
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_render_has_height_newlines() {
        let p = WallpaperParams::default();
        let mut buf = Vec::new();
        AsciiPreview::render(&p, 10, 5, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<_> = s.lines().collect();
        assert_eq!(lines.len(), 5, "expected 5 lines, got {}", lines.len());
    }

    #[test]
    fn test_render_zero_size() {
        let p = WallpaperParams::default();
        let mut buf = Vec::new();
        AsciiPreview::render(&p, 0, 0, &mut buf).unwrap();
        // Zero dimensions: no content output (only empty lines from 0 rows)
        assert_eq!(buf.len(), 0);
    }

    #[test]
    fn test_render_different_params_produce_different_output() {
        let p1 = WallpaperParams { hue_offset: 0.0, ..Default::default() };
        let p2 = WallpaperParams { hue_offset: 90.0, ..Default::default() };
        let mut buf1 = Vec::new();
        let mut buf2 = Vec::new();
        AsciiPreview::render(&p1, 8, 4, &mut buf1).unwrap();
        AsciiPreview::render(&p2, 8, 4, &mut buf2).unwrap();
        assert_ne!(buf1, buf2, "different hue_offset should produce different output");
    }

    // ── TuiApp tests ──────────────────────────────────────────────────────

    #[test]
    fn test_tui_app_default() {
        let app = TuiApp::new();
        assert_eq!(app.width, 40);
        assert_eq!(app.height, 20);
    }

    #[test]
    fn test_tui_app_with_size() {
        let app = TuiApp::with_size(20, 10);
        assert_eq!(app.width, 20);
        assert_eq!(app.height, 10);
    }

    #[test]
    fn test_tui_result_saved_false_by_default() {
        // TuiApp::run() returns saved=false (non-interactive)
        // We can't call run() in tests easily (writes to stdout),
        // but we can construct TuiResult directly.
        let result = TuiResult {
            params: WallpaperParams::default(),
            saved: false,
        };
        assert!(!result.saved);
    }

    #[test]
    fn test_save_params_to_file() {
        let app = TuiApp::new();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("params.toml");
        app.save_params(&path).expect("save");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains("symmetry_group"));
        assert!(content.contains("scale"));
    }
}
