//! Phase-portrait recording: captures frames and assembles them into a GIF.
//!
//! # Usage
//!
//! Press **R** while the wallpaper window has focus to start/stop recording.
//! By default 10 seconds of animation are captured at 30 fps, producing up to
//! 300 PNG frames in a temporary directory, then assembled into
//! `geodesic-recording.gif` in the current working directory.
//!
//! # Architecture
//!
//! [`PhasePortraitRecorder`] manages recording state.  The main render loop
//! calls [`PhasePortraitRecorder::push_frame`] each frame; the recorder
//! accumulates raw RGBA bytes.  When the desired duration has elapsed (or the
//! user presses R again) [`PhasePortraitRecorder::finish`] is called, which
//! writes the frames to disk and then calls [`encode_to_gif`].
//!
//! Frame encoding uses the `image` crate (already a dependency).

use image::{ImageBuffer, Rgba, RgbaImage};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ─── Error ────────────────────────────────────────────────────────────────────

/// Errors that can occur during recording or GIF encoding.
#[derive(Debug)]
pub enum RecorderError {
    /// An IO operation failed.
    Io(io::Error),
    /// The `image` crate reported an error.
    Image(image::ImageError),
    /// The frames directory was not set when encoding was attempted.
    NoFramesDir,
    /// Width or height was zero.
    InvalidDimensions,
}

impl std::fmt::Display for RecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecorderError::Io(e) => write!(f, "IO error: {e}"),
            RecorderError::Image(e) => write!(f, "image error: {e}"),
            RecorderError::NoFramesDir => write!(f, "no frames directory available"),
            RecorderError::InvalidDimensions => write!(f, "invalid frame dimensions (0)"),
        }
    }
}

impl std::error::Error for RecorderError {}

impl From<io::Error> for RecorderError {
    fn from(e: io::Error) -> Self {
        RecorderError::Io(e)
    }
}

impl From<image::ImageError> for RecorderError {
    fn from(e: image::ImageError) -> Self {
        RecorderError::Image(e)
    }
}

// ─── Recording state ──────────────────────────────────────────────────────────

/// Whether the recorder is currently active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingState {
    /// Not recording.
    Idle,
    /// Actively capturing frames.
    Recording,
    /// Finished capturing; encoding in progress.
    Encoding,
}

// ─── PhasePortraitRecorder ────────────────────────────────────────────────────

/// Captures frames of the geodesic animation and assembles them into a GIF.
pub struct PhasePortraitRecorder {
    /// Current state of the recorder.
    state: RecordingState,
    /// Frame width in pixels.
    width: u32,
    /// Frame height in pixels.
    height: u32,
    /// Target frames per second.
    fps: u32,
    /// Maximum recording duration.
    max_duration: Duration,
    /// Wall-clock time when recording started.
    start_time: Option<Instant>,
    /// Temporary directory that holds PNG frames.
    frames_dir: Option<PathBuf>,
    /// Number of frames written so far.
    frame_count: usize,
    /// Output GIF path.
    output_path: PathBuf,
}

impl PhasePortraitRecorder {
    /// Create a new recorder.
    ///
    /// - `fps`: target capture rate (default 30 if 0 is passed).
    /// - `max_secs`: how many seconds to record (default 10 if 0 is passed).
    /// - `output`: where to write the final GIF.
    pub fn new(width: u32, height: u32, fps: u32, max_secs: u32, output: PathBuf) -> Self {
        let fps = if fps == 0 { 30 } else { fps };
        let max_secs = if max_secs == 0 { 10 } else { max_secs };
        Self {
            state: RecordingState::Idle,
            width,
            height,
            fps,
            max_duration: Duration::from_secs(max_secs as u64),
            start_time: None,
            frames_dir: None,
            frame_count: 0,
            output_path: output,
        }
    }

    /// The current recording state.
    pub fn state(&self) -> RecordingState {
        self.state
    }

    /// Returns `true` if the recorder is actively capturing.
    pub fn is_recording(&self) -> bool {
        self.state == RecordingState::Recording
    }

    /// Number of frames captured so far.
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Start recording.  Creates the temporary frames directory.
    ///
    /// If recording is already in progress this is a no-op.
    pub fn start(&mut self) -> Result<(), RecorderError> {
        if self.state == RecordingState::Recording {
            return Ok(());
        }
        if self.width == 0 || self.height == 0 {
            return Err(RecorderError::InvalidDimensions);
        }
        let dir = std::env::temp_dir().join(format!(
            "geodesic-rec-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        std::fs::create_dir_all(&dir)?;
        self.frames_dir = Some(dir);
        self.frame_count = 0;
        self.start_time = Some(Instant::now());
        self.state = RecordingState::Recording;
        tracing::info!(
            "Recording started: {}x{} @ {}fps, max {}s → {}",
            self.width,
            self.height,
            self.fps,
            self.max_duration.as_secs(),
            self.output_path.display()
        );
        Ok(())
    }

    /// Push one RGBA frame.  Ignored unless `state == Recording`.
    ///
    /// `rgba_bytes` must be exactly `width * height * 4` bytes.
    /// Stops recording automatically when the time limit is reached.
    pub fn push_frame(&mut self, rgba_bytes: &[u8]) -> Result<(), RecorderError> {
        if self.state != RecordingState::Recording {
            return Ok(());
        }

        // Check duration limit.
        if let Some(start) = self.start_time {
            if start.elapsed() >= self.max_duration {
                tracing::info!("Recording duration limit reached, stopping automatically");
                self.state = RecordingState::Idle; // mark idle so finish() re-enters
                return Ok(()); // caller should call finish()
            }
        }

        let expected = (self.width * self.height * 4) as usize;
        if rgba_bytes.len() != expected {
            tracing::warn!(
                "push_frame: expected {} bytes, got {}; skipping frame",
                expected,
                rgba_bytes.len()
            );
            return Ok(());
        }

        let dir = self.frames_dir.as_ref().ok_or(RecorderError::NoFramesDir)?;
        let frame_path = dir.join(format!("frame_{:06}.png", self.frame_count));

        let img: RgbaImage = ImageBuffer::from_raw(self.width, self.height, rgba_bytes.to_vec())
            .ok_or(RecorderError::InvalidDimensions)?;
        img.save(&frame_path)?;

        self.frame_count += 1;
        Ok(())
    }

    /// Stop recording and encode the captured frames to a GIF.
    ///
    /// Returns the path of the output GIF on success.
    pub fn finish(&mut self) -> Result<PathBuf, RecorderError> {
        self.state = RecordingState::Encoding;
        let dir = self.frames_dir.take().ok_or(RecorderError::NoFramesDir)?;
        let output = self.output_path.clone();
        tracing::info!(
            "Encoding {} frames from {} to {}",
            self.frame_count,
            dir.display(),
            output.display()
        );
        encode_to_gif(&dir, &output, self.fps)?;
        // Clean up the temp directory (best-effort).
        let _ = std::fs::remove_dir_all(&dir);
        self.state = RecordingState::Idle;
        self.frame_count = 0;
        Ok(output)
    }

    /// Toggle recording on/off.
    ///
    /// - If idle → starts recording.
    /// - If recording → stops and encodes.
    /// - If encoding → no-op.
    ///
    /// Returns the output path when encoding completes, `None` otherwise.
    pub fn toggle(&mut self) -> Result<Option<PathBuf>, RecorderError> {
        match self.state {
            RecordingState::Idle => {
                self.start()?;
                Ok(None)
            }
            RecordingState::Recording => {
                let path = self.finish()?;
                Ok(Some(path))
            }
            RecordingState::Encoding => Ok(None),
        }
    }

    /// A short status string suitable for display in the window title.
    pub fn status_text(&self) -> String {
        match self.state {
            RecordingState::Idle => String::new(),
            RecordingState::Recording => {
                let elapsed = self.start_time
                    .map(|s| s.elapsed().as_secs())
                    .unwrap_or(0);
                format!(" [REC {}s / {}s  {} frames]",
                    elapsed,
                    self.max_duration.as_secs(),
                    self.frame_count)
            }
            RecordingState::Encoding => format!(" [ENCODING {} frames…]", self.frame_count),
        }
    }
}

// ─── GIF encoder ─────────────────────────────────────────────────────────────

/// Assemble PNG frames from `frames_dir` into a GIF at `output`.
///
/// Frames are read in lexicographic order (as written by [`PhasePortraitRecorder`]).
/// Each frame is converted to an 8-bit paletted image using a simple per-frame
/// median-cut quantisation via `image`'s built-in facilities.
///
/// # Arguments
/// - `frames_dir`: directory containing `frame_000000.png` … files.
/// - `output`: path for the resulting `.gif` file.
/// - `fps`: playback speed; each GIF frame delay = `100 / fps` centiseconds.
pub fn encode_to_gif(frames_dir: &Path, output: &Path, fps: u32) -> Result<(), RecorderError> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};

    let fps = fps.max(1);
    let delay = Delay::from_numer_denom_ms(1000, fps);

    // Collect frame paths in sorted order.
    let mut paths: Vec<PathBuf> = std::fs::read_dir(frames_dir)?
        .filter_map(|e| e.ok().map(|de| de.path()))
        .filter(|p| p.extension().map_or(false, |ext| ext == "png"))
        .collect();
    paths.sort();

    if paths.is_empty() {
        tracing::warn!("encode_to_gif: no PNG frames found in {}", frames_dir.display());
        return Ok(());
    }

    let out_file = std::fs::File::create(output)?;
    let mut encoder = GifEncoder::new(out_file);
    encoder.set_repeat(Repeat::Infinite).map_err(RecorderError::Image)?;

    for path in &paths {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let frame = Frame::from_parts(rgba, 0, 0, delay);
        encoder.encode_frame(frame).map_err(RecorderError::Image)?;
    }

    tracing::info!("GIF written to {}", output.display());
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_recorder(w: u32, h: u32) -> PhasePortraitRecorder {
        PhasePortraitRecorder::new(
            w, h, 30, 10,
            PathBuf::from("test-output.gif"),
        )
    }

    #[test]
    fn initial_state_idle() {
        let r = make_recorder(64, 64);
        assert_eq!(r.state(), RecordingState::Idle);
        assert_eq!(r.frame_count(), 0);
    }

    #[test]
    fn start_changes_state() {
        let mut r = make_recorder(64, 64);
        r.start().unwrap();
        assert_eq!(r.state(), RecordingState::Recording);
    }

    #[test]
    fn push_frame_wrong_size_skipped() {
        let mut r = make_recorder(64, 64);
        r.start().unwrap();
        // Intentionally wrong size — should not error, just skip.
        let result = r.push_frame(&[0u8; 16]);
        assert!(result.is_ok());
        assert_eq!(r.frame_count(), 0, "frame should be skipped");
    }

    #[test]
    fn push_frame_correct_size_stored() {
        let mut r = make_recorder(4, 4);
        r.start().unwrap();
        let frame = vec![128u8; 4 * 4 * 4];
        r.push_frame(&frame).unwrap();
        assert_eq!(r.frame_count(), 1);
    }

    #[test]
    fn zero_dimensions_returns_error() {
        let mut r = make_recorder(0, 64);
        let result = r.start();
        assert!(matches!(result, Err(RecorderError::InvalidDimensions)));
    }

    #[test]
    fn status_text_idle_is_empty() {
        let r = make_recorder(64, 64);
        assert!(r.status_text().is_empty());
    }

    #[test]
    fn status_text_recording_non_empty() {
        let mut r = make_recorder(4, 4);
        r.start().unwrap();
        let text = r.status_text();
        assert!(text.contains("REC"), "status should mention REC");
    }

    #[test]
    fn encode_to_gif_empty_dir_is_ok() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("out.gif");
        // Empty directory — should return Ok without writing anything.
        encode_to_gif(dir.path(), &out, 30).unwrap();
    }

    #[test]
    fn encode_to_gif_single_frame() {
        let dir = tempdir().unwrap();
        // Write a tiny 2×2 RGBA PNG.
        let img: RgbaImage = ImageBuffer::from_fn(2, 2, |x, y| {
            Rgba([(x * 127) as u8, (y * 127) as u8, 64, 255])
        });
        img.save(dir.path().join("frame_000000.png")).unwrap();

        let out = dir.path().join("out.gif");
        encode_to_gif(dir.path(), &out, 10).unwrap();
        assert!(out.exists(), "GIF should be created");
        assert!(out.metadata().unwrap().len() > 0, "GIF should be non-empty");
    }

    #[test]
    fn full_record_and_encode() {
        let dir = tempdir().unwrap();
        let out = dir.path().join("rec.gif");
        let mut r = PhasePortraitRecorder::new(2, 2, 30, 1, out.clone());
        r.start().unwrap();
        let frame = vec![255u8; 2 * 2 * 4];
        r.push_frame(&frame).unwrap();
        let result = r.finish().unwrap();
        assert_eq!(result, out);
        assert!(out.exists());
    }
}
