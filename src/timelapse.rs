//! Time-lapse frame recorder and FFmpeg video compiler.
//!
//! Captures wallpaper frames at configurable intervals and compiles them into
//! an MP4 video file by spawning an `ffmpeg` subprocess.
//!
//! ## Usage (CLI)
//!
//! ```text
//! geodesic-wallpaper --timelapse 24h
//! ```
//!
//! This records one frame per minute for 24 hours (1440 frames total) and
//! saves them under `timelapse/frame_NNNNNN.png`. When the session ends (or
//! `--timelapse-compile` is called) the frames are compiled into
//! `timelapse_output.mp4` at the requested output frame rate.
//!
//! ## Module Overview
//!
//! - [`TimelapseDuration`] — parses duration strings like `"24h"`, `"30m"`,
//!   `"90s"`, `"1h30m"`.
//! - [`TimelapseConfig`] — full recording configuration.
//! - [`TimelapseRecorder`] — tracks elapsed time, decides when to capture,
//!   and writes PNG frames via a user-supplied callback.
//! - [`compile_to_mp4`] — spawns `ffmpeg` to stitch frames into MP4.
//!
//! ## Frame Capture
//!
//! Frame capture is decoupled from rendering: the caller provides a
//! `capture_fn: impl Fn(usize) -> Vec<u8>` that returns raw RGBA pixels for
//! the frame. [`TimelapseRecorder::tick`] calls this function only when the
//! configured interval has elapsed.
//!
//! The PNG encoder is implemented as a minimal stdlib-only writer (no external
//! crate needed for the test suite).

use std::path::{Path, PathBuf};
use std::time::Duration;

/// A parsed capture duration: total number of seconds to record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelapseDuration {
    /// Total wall-clock seconds to record.
    pub total_seconds: u64,
}

impl TimelapseDuration {
    /// Parse a human-readable duration string.
    ///
    /// Supported formats: `"24h"`, `"30m"`, `"90s"`, `"1h30m"`, `"2h15m30s"`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        if s.is_empty() {
            return Err("empty duration string".to_string());
        }

        let mut total = 0u64;
        let mut num_buf = String::new();

        for ch in s.chars() {
            if ch.is_ascii_digit() {
                num_buf.push(ch);
            } else {
                let n: u64 = num_buf
                    .parse()
                    .map_err(|e| format!("invalid number: {e}"))?;
                num_buf.clear();
                match ch {
                    'h' | 'H' => total += n * 3600,
                    'm' | 'M' => total += n * 60,
                    's' | 'S' => total += n,
                    other => return Err(format!("unexpected unit character '{other}'")),
                }
            }
        }

        if !num_buf.is_empty() {
            return Err(format!("trailing number without unit: '{num_buf}'"));
        }
        if total == 0 {
            return Err("duration must be greater than zero".to_string());
        }

        Ok(Self {
            total_seconds: total,
        })
    }

    /// Duration as a [`std::time::Duration`].
    pub fn as_std_duration(self) -> Duration {
        Duration::from_secs(self.total_seconds)
    }

    /// Human-readable representation.
    pub fn display(self) -> String {
        let h = self.total_seconds / 3600;
        let m = (self.total_seconds % 3600) / 60;
        let s = self.total_seconds % 60;
        if h > 0 {
            format!("{h}h{m:02}m{s:02}s")
        } else {
            format!("{m}m{s:02}s")
        }
    }
}

/// Configuration for the time-lapse recorder.
#[derive(Debug, Clone)]
pub struct TimelapseConfig {
    /// How long to record for.
    pub duration: TimelapseDuration,
    /// Interval between captured frames.
    pub capture_interval: Duration,
    /// Directory where PNG frames are written.
    pub output_dir: PathBuf,
    /// Output MP4 filename (relative to `output_dir`).
    pub output_filename: PathBuf,
    /// Frame rate of the compiled MP4 (frames per second).
    pub fps: u32,
    /// Width in pixels of each captured frame.
    pub frame_width: u32,
    /// Height in pixels of each captured frame.
    pub frame_height: u32,
}

impl TimelapseConfig {
    /// Create a configuration for a `--timelapse <duration>` invocation.
    ///
    /// Defaults to one frame per minute, 30 fps output.
    pub fn from_duration_str(s: &str) -> Result<Self, String> {
        let duration = TimelapseDuration::parse(s)?;
        Ok(Self {
            duration,
            capture_interval: Duration::from_secs(60),
            output_dir: PathBuf::from("timelapse"),
            output_filename: PathBuf::from("timelapse_output.mp4"),
            fps: 30,
            frame_width: 1920,
            frame_height: 1080,
        })
    }

    /// Total expected frame count.
    pub fn expected_frames(&self) -> u64 {
        self.duration.total_seconds / self.capture_interval.as_secs().max(1)
    }
}

impl Default for TimelapseConfig {
    fn default() -> Self {
        Self::from_duration_str("24h").expect("default duration is valid")
    }
}

/// Time-lapse recorder state.
///
/// Drive by calling [`TimelapseRecorder::tick`] on every render frame,
/// passing the elapsed `dt` since the previous call. When a frame capture
/// interval has elapsed, the supplied closure is invoked.
pub struct TimelapseRecorder {
    cfg: TimelapseConfig,
    /// Elapsed time since last frame capture.
    time_since_capture: Duration,
    /// Elapsed total recording time.
    total_elapsed: Duration,
    /// Index of the next frame to be captured.
    pub frame_index: usize,
    /// Whether recording has finished.
    pub finished: bool,
    /// Paths of all frames written so far.
    frame_paths: Vec<PathBuf>,
}

impl TimelapseRecorder {
    /// Create a new recorder. Does **not** create the output directory yet.
    pub fn new(cfg: TimelapseConfig) -> Self {
        Self {
            time_since_capture: Duration::ZERO,
            total_elapsed: Duration::ZERO,
            frame_index: 0,
            finished: false,
            frame_paths: Vec::new(),
            cfg,
        }
    }

    /// Return the recorder configuration.
    pub fn config(&self) -> &TimelapseConfig {
        &self.cfg
    }

    /// All frame paths written so far.
    pub fn frame_paths(&self) -> &[PathBuf] {
        &self.frame_paths
    }

    /// Advance the recorder by `dt`. When the capture interval elapses,
    /// `capture_fn(frame_index)` is called; the returned raw RGBA pixel data
    /// is written to a PNG file.
    ///
    /// Returns `true` if a frame was captured on this tick.
    pub fn tick<F>(&mut self, dt: Duration, capture_fn: F) -> bool
    where
        F: FnOnce(usize) -> Vec<u8>,
    {
        if self.finished {
            return false;
        }

        self.time_since_capture += dt;
        self.total_elapsed += dt;

        // Check recording duration.
        if self.total_elapsed >= self.cfg.duration.as_std_duration() {
            self.finished = true;
        }

        if self.time_since_capture >= self.cfg.capture_interval {
            self.time_since_capture = Duration::ZERO;
            let rgba = capture_fn(self.frame_index);
            let path = self.frame_path(self.frame_index);
            if let Err(e) =
                write_png_rgba(&path, &rgba, self.cfg.frame_width, self.cfg.frame_height)
            {
                eprintln!(
                    "[timelapse] failed to write frame {}: {e}",
                    self.frame_index
                );
            } else {
                self.frame_paths.push(path);
            }
            self.frame_index += 1;
            return true;
        }

        false
    }

    /// Construct the path for frame `n`.
    pub fn frame_path(&self, n: usize) -> PathBuf {
        self.cfg.output_dir.join(format!("frame_{n:06}.png"))
    }

    /// Progress as a fraction in `[0, 1]`.
    pub fn progress(&self) -> f32 {
        let total_secs = self.cfg.duration.total_seconds as f32;
        if total_secs <= 0.0 {
            return 1.0;
        }
        (self.total_elapsed.as_secs_f32() / total_secs).clamp(0.0, 1.0)
    }

    /// Human-readable status string.
    pub fn status(&self) -> String {
        format!(
            "frame {}/{} ({:.1}%)",
            self.frame_index,
            self.cfg.expected_frames(),
            self.progress() * 100.0,
        )
    }
}

/// Spawn `ffmpeg` to compile captured frames into an MP4.
///
/// Requires `ffmpeg` to be available on `PATH`. The frames must have been
/// written to `output_dir/frame_NNNNNN.png` (the default output of
/// [`TimelapseRecorder`]).
///
/// The compiled video is written to `output_dir/output_filename`.
///
/// Returns the exit status string on success or an error message.
pub fn compile_to_mp4(cfg: &TimelapseConfig) -> Result<String, String> {
    let input_pattern = cfg
        .output_dir
        .join("frame_%06d.png")
        .to_string_lossy()
        .into_owned();
    let output_path = cfg
        .output_dir
        .join(&cfg.output_filename)
        .to_string_lossy()
        .into_owned();

    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y", // overwrite output
            "-framerate",
            &cfg.fps.to_string(),
            "-i",
            &input_pattern,
            "-c:v",
            "libx264",
            "-crf",
            "18",
            "-pix_fmt",
            "yuv420p",
            &output_path,
        ])
        .status()
        .map_err(|e| format!("failed to spawn ffmpeg: {e}"))?;

    if status.success() {
        Ok(format!(
            "compiled {} frames to {output_path}",
            cfg.expected_frames()
        ))
    } else {
        Err(format!("ffmpeg exited with status: {status}"))
    }
}

// ── PNG writer ────────────────────────────────────────────────────────────────

/// Write raw RGBA pixel data as a compressed PNG (via the `image` crate).
///
/// Fails, instead of writing a corrupt file, when `rgba` is not exactly
/// `width * height * 4` bytes.
fn write_png_rgba(path: &Path, rgba: &[u8], width: u32, height: u32) -> Result<(), String> {
    crate::export::save_rgba_png(path, width, height, rgba)
        .map(|_| ())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TimelapseDuration ─────────────────────────────────────────────────────

    #[test]
    fn parse_hours() {
        let d = TimelapseDuration::parse("24h").unwrap();
        assert_eq!(d.total_seconds, 24 * 3600);
    }

    #[test]
    fn parse_minutes() {
        let d = TimelapseDuration::parse("90m").unwrap();
        assert_eq!(d.total_seconds, 90 * 60);
    }

    #[test]
    fn parse_seconds() {
        let d = TimelapseDuration::parse("300s").unwrap();
        assert_eq!(d.total_seconds, 300);
    }

    #[test]
    fn parse_compound() {
        let d = TimelapseDuration::parse("1h30m").unwrap();
        assert_eq!(d.total_seconds, 3600 + 1800);
    }

    #[test]
    fn parse_compound_with_seconds() {
        let d = TimelapseDuration::parse("2h15m30s").unwrap();
        assert_eq!(d.total_seconds, 2 * 3600 + 15 * 60 + 30);
    }

    #[test]
    fn parse_invalid_returns_error() {
        assert!(TimelapseDuration::parse("").is_err());
        assert!(TimelapseDuration::parse("5x").is_err());
        assert!(TimelapseDuration::parse("0h").is_err());
    }

    // ── TimelapseConfig ───────────────────────────────────────────────────────

    #[test]
    fn expected_frames_correct() {
        let mut cfg = TimelapseConfig::from_duration_str("1h").unwrap();
        cfg.capture_interval = Duration::from_secs(60);
        assert_eq!(cfg.expected_frames(), 60);
    }

    // ── TimelapseRecorder ─────────────────────────────────────────────────────

    #[test]
    fn tick_does_not_capture_before_interval() {
        let mut cfg = TimelapseConfig::from_duration_str("1h").unwrap();
        cfg.output_dir = std::env::temp_dir().join("gwp_test_nocapture");
        cfg.capture_interval = Duration::from_secs(60);
        cfg.frame_width = 2;
        cfg.frame_height = 2;
        let mut rec = TimelapseRecorder::new(cfg);
        let captured = rec.tick(Duration::from_secs(30), |_| vec![0u8; 2 * 2 * 4]);
        assert!(!captured);
        assert_eq!(rec.frame_index, 0);
    }

    #[test]
    fn tick_captures_after_interval() {
        let mut cfg = TimelapseConfig::from_duration_str("1h").unwrap();
        let tmp = std::env::temp_dir().join("gwp_test_capture");
        cfg.output_dir = tmp;
        cfg.capture_interval = Duration::from_secs(60);
        cfg.frame_width = 2;
        cfg.frame_height = 2;
        let mut rec = TimelapseRecorder::new(cfg);
        let captured = rec.tick(Duration::from_secs(60), |_| vec![128u8; 2 * 2 * 4]);
        assert!(captured);
        assert_eq!(rec.frame_index, 1);
    }

    #[test]
    fn finished_after_duration_elapsed() {
        let mut cfg = TimelapseConfig::from_duration_str("5s").unwrap();
        cfg.output_dir = std::env::temp_dir().join("gwp_test_finished");
        cfg.capture_interval = Duration::from_secs(1);
        cfg.frame_width = 2;
        cfg.frame_height = 2;
        let mut rec = TimelapseRecorder::new(cfg);
        rec.tick(Duration::from_secs(10), |_| vec![0u8; 16]);
        assert!(rec.finished);
    }

    #[test]
    fn progress_increases() {
        let cfg = TimelapseConfig::from_duration_str("10s").unwrap();
        let mut rec = TimelapseRecorder::new(cfg);
        rec.total_elapsed = Duration::from_secs(5);
        let p = rec.progress();
        assert!((p - 0.5).abs() < 0.01);
    }

    #[test]
    fn status_string_is_non_empty() {
        let cfg = TimelapseConfig::from_duration_str("1h").unwrap();
        let rec = TimelapseRecorder::new(cfg);
        assert!(!rec.status().is_empty());
    }

    #[test]
    fn frame_path_formatting() {
        let cfg = TimelapseConfig::from_duration_str("1h").unwrap();
        let rec = TimelapseRecorder::new(cfg);
        let p = rec.frame_path(42);
        assert!(p.to_string_lossy().contains("frame_000042.png"));
    }

    #[test]
    fn duration_display() {
        let d = TimelapseDuration::parse("1h30m").unwrap();
        let s = d.display();
        assert!(s.contains('h'));
    }
}
