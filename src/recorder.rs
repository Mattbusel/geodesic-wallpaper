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

use image::{ImageBuffer, RgbaImage};
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
        // Unique per process and per recording, so two recorders started in
        // the same second never share (and overwrite) a frames directory.
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "geodesic-rec-{}-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            std::process::id(),
            NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
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
                let elapsed = self.start_time.map(|s| s.elapsed().as_secs()).unwrap_or(0);
                format!(
                    " [REC {}s / {}s  {} frames]",
                    elapsed,
                    self.max_duration.as_secs(),
                    self.frame_count
                )
            }
            RecordingState::Encoding => format!(" [ENCODING {} frames…]", self.frame_count),
        }
    }
}

// ─── GIF encoder ─────────────────────────────────────────────────────────────

/// GIF frame delay in hundredths of a second for `fps`, rounded to the nearest
/// value GIF can store.
///
/// GIF stores delays in centiseconds and browsers treat delays under 2 cs as
/// 10 cs, so the result is clamped to at least 2 (50 fps).
pub fn gif_delay_centis(fps: u32) -> u16 {
    let fps = fps.max(1) as f64;
    ((100.0 / fps).round() as u16).max(2)
}

/// Streams RGBA frames straight into an animated, looping GIF file.
///
/// Each frame is quantised to a 256-colour palette by the `gif` crate
/// (NeuQuant). Nothing is buffered on disk, so long recordings only cost the
/// size of the output file.
///
/// ```
/// use geodesic_wallpaper::recorder::GifWriter;
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("spin.gif");
/// let mut gif = GifWriter::create(&path, 4, 4, 15).unwrap();
/// gif.push_rgba(&[255u8; 4 * 4 * 4]).unwrap();
/// gif.push_rgba(&[0u8; 4 * 4 * 4]).unwrap();
/// assert_eq!(gif.finish().unwrap(), 2);
/// ```
pub struct GifWriter {
    encoder: image::codecs::gif::GifEncoder<io::BufWriter<std::fs::File>>,
    width: u32,
    height: u32,
    delay: image::Delay,
    frames: usize,
    path: PathBuf,
}

impl GifWriter {
    /// Speed passed to the NeuQuant quantiser: 1 is best and slowest, 30 is
    /// fastest. 10 is the `gif` crate's recommended balance.
    pub const QUANTIZE_SPEED: i32 = 10;

    /// Create `path` (and its parent folders) and start a looping GIF of
    /// `width` x `height` frames played at `fps`.
    pub fn create(path: &Path, width: u32, height: u32, fps: u32) -> Result<Self, RecorderError> {
        use image::codecs::gif::{GifEncoder, Repeat};
        if width == 0 || height == 0 || width > u16::MAX as u32 || height > u16::MAX as u32 {
            return Err(RecorderError::InvalidDimensions);
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = io::BufWriter::new(std::fs::File::create(path)?);
        let mut encoder = GifEncoder::new_with_speed(file, Self::QUANTIZE_SPEED);
        encoder.set_repeat(Repeat::Infinite)?;
        let centis = gif_delay_centis(fps) as u32;
        Ok(Self {
            encoder,
            width,
            height,
            delay: image::Delay::from_numer_denom_ms(centis * 10, 1),
            frames: 0,
            path: path.to_path_buf(),
        })
    }

    /// Append one frame of tightly packed RGBA8 pixels
    /// (`width * height * 4` bytes).
    pub fn push_rgba(&mut self, rgba: &[u8]) -> Result<(), RecorderError> {
        let img: RgbaImage = ImageBuffer::from_raw(self.width, self.height, rgba.to_vec())
            .ok_or(RecorderError::InvalidDimensions)?;
        self.encoder
            .encode_frame(image::Frame::from_parts(img, 0, 0, self.delay))?;
        self.frames += 1;
        Ok(())
    }

    /// Number of frames written so far.
    pub fn frame_count(&self) -> usize {
        self.frames
    }

    /// Close the file and return the number of frames written.
    ///
    /// The GIF trailer is written when the encoder is dropped, which cannot
    /// report errors, so the file is read back to confirm it ends with the
    /// trailer byte.
    pub fn finish(self) -> Result<usize, RecorderError> {
        let frames = self.frames;
        let path = self.path;
        drop(self.encoder);
        let bytes = std::fs::read(&path)?;
        if bytes.last() != Some(&0x3B) {
            return Err(RecorderError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("{} was not fully written", path.display()),
            )));
        }
        Ok(frames)
    }
}

/// Writes RGBA frames into an animated PNG (APNG) that loops forever.
///
/// Unlike GIF, APNG keeps full 24-bit colour and 8-bit alpha, so the glow on
/// the trails does not band. Every major browser plays APNG. The number of
/// frames is written into the file header, so it must be known up front.
///
/// ```
/// use geodesic_wallpaper::recorder::ApngWriter;
/// let dir = tempfile::tempdir().unwrap();
/// let path = dir.path().join("spin.png");
/// let mut apng = ApngWriter::create(&path, 4, 4, 30, 2).unwrap();
/// apng.push_rgba(&[255u8; 4 * 4 * 4]).unwrap();
/// apng.push_rgba(&[0u8; 4 * 4 * 4]).unwrap();
/// assert_eq!(apng.finish().unwrap(), 2);
/// ```
pub struct ApngWriter {
    writer: png::Writer<io::BufWriter<std::fs::File>>,
    width: u32,
    height: u32,
    expected: u32,
    frames: u32,
}

impl ApngWriter {
    /// Create `path` (and its parent folders) for `frame_count` frames of
    /// `width` x `height` pixels, played at `fps`.
    pub fn create(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        frame_count: u32,
    ) -> Result<Self, RecorderError> {
        if width == 0 || height == 0 || frame_count == 0 {
            return Err(RecorderError::InvalidDimensions);
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = io::BufWriter::new(std::fs::File::create(path)?);
        let mut enc = png::Encoder::new(file, width, height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Balanced);
        enc.set_animated(frame_count, 0).map_err(png_err)?;
        let fps = fps.clamp(1, u16::MAX as u32) as u16;
        enc.set_frame_delay(1, fps).map_err(png_err)?;
        let writer = enc.write_header().map_err(png_err)?;
        Ok(Self {
            writer,
            width,
            height,
            expected: frame_count,
            frames: 0,
        })
    }

    /// Append one frame of tightly packed RGBA8 pixels
    /// (`width * height * 4` bytes). Fails once `frame_count` frames have
    /// been written.
    pub fn push_rgba(&mut self, rgba: &[u8]) -> Result<(), RecorderError> {
        if rgba.len() as u64 != self.width as u64 * self.height as u64 * 4 {
            return Err(RecorderError::InvalidDimensions);
        }
        if self.frames >= self.expected {
            return Err(RecorderError::Io(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("APNG was created for {} frames", self.expected),
            )));
        }
        self.writer.write_image_data(rgba).map_err(png_err)?;
        self.frames += 1;
        Ok(())
    }

    /// Number of frames written so far.
    pub fn frame_count(&self) -> u32 {
        self.frames
    }

    /// Finish the file. Fails if fewer frames were written than declared.
    pub fn finish(self) -> Result<u32, RecorderError> {
        let frames = self.frames;
        if frames != self.expected {
            return Err(RecorderError::Io(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("APNG expected {} frames, got {frames}", self.expected),
            )));
        }
        self.writer.finish().map_err(png_err)?;
        Ok(frames)
    }
}

fn png_err(e: png::EncodingError) -> RecorderError {
    match e {
        png::EncodingError::IoError(io) => RecorderError::Io(io),
        other => RecorderError::Io(io::Error::other(other.to_string())),
    }
}

/// Streams RGBA frames into an H.264 MP4 through an external `ffmpeg`.
///
/// ffmpeg is a free tool that is not bundled: if it is not on `PATH` (or at
/// the path given), [`FfmpegWriter::spawn`] returns a clear error. Frames are
/// piped as raw RGBA, so nothing touches the disk except the output video.
/// Odd sizes are padded by one pixel because H.264 4:2:0 needs even sizes.
pub struct FfmpegWriter {
    child: std::process::Child,
    stdin: Option<std::process::ChildStdin>,
    frame_bytes: usize,
    frames: usize,
}

impl FfmpegWriter {
    /// Start `ffmpeg` (or the executable at `ffmpeg`) writing `path` at
    /// `fps` frames per second.
    pub fn spawn(
        ffmpeg: &str,
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
    ) -> Result<Self, RecorderError> {
        use std::process::{Command, Stdio};
        if width == 0 || height == 0 {
            return Err(RecorderError::InvalidDimensions);
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut cmd = Command::new(ffmpeg);
        cmd.args(["-hide_banner", "-loglevel", "error", "-y"])
            .args(["-f", "rawvideo", "-pix_fmt", "rgba"])
            .args(["-s", &format!("{width}x{height}")])
            .args(["-r", &fps.max(1).to_string()])
            .args(["-i", "-"])
            .args(["-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2"])
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "18"])
            .args(["-movflags", "+faststart"])
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = cmd.spawn().map_err(|e| {
            RecorderError::Io(io::Error::new(
                e.kind(),
                format!(
                    "could not start {ffmpeg} ({e}). Install ffmpeg (https://ffmpeg.org, or `winget install Gyan.FFmpeg`) or use --gif / --apng"
                ),
            ))
        })?;
        let stdin = child.stdin.take();
        Ok(Self {
            child,
            stdin,
            frame_bytes: width as usize * height as usize * 4,
            frames: 0,
        })
    }

    /// Append one frame of RGBA8 pixels (`width * height * 4` bytes).
    pub fn push_rgba(&mut self, rgba: &[u8]) -> Result<(), RecorderError> {
        if rgba.len() != self.frame_bytes {
            return Err(RecorderError::InvalidDimensions);
        }
        let stdin = self.stdin.as_mut().ok_or(RecorderError::NoFramesDir)?;
        io::Write::write_all(stdin, rgba)?;
        self.frames += 1;
        Ok(())
    }

    /// Close the pipe, wait for ffmpeg and return the number of frames.
    /// ffmpeg's own error text is included if it fails.
    pub fn finish(mut self) -> Result<usize, RecorderError> {
        drop(self.stdin.take());
        let out = self.child.wait_with_output()?;
        if !out.status.success() {
            return Err(RecorderError::Io(io::Error::other(format!(
                "ffmpeg failed ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ))));
        }
        Ok(self.frames)
    }
}

/// Assemble PNG frames from `frames_dir` into a looping GIF at `output`.
///
/// Frames are read in file-name order (as written by
/// [`PhasePortraitRecorder`] or `geodesic-wallpaper --record`). Every frame
/// must have the size of the first one.
///
/// # Arguments
/// - `frames_dir`: directory containing `frame_000000.png` and so on.
/// - `output`: path for the resulting `.gif` file.
/// - `fps`: playback speed, see [`gif_delay_centis`].
pub fn encode_to_gif(frames_dir: &Path, output: &Path, fps: u32) -> Result<(), RecorderError> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(frames_dir)?
        .filter_map(|e| e.ok().map(|de| de.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "png"))
        .collect();
    paths.sort();

    let Some(first) = paths.first() else {
        tracing::warn!(
            "encode_to_gif: no PNG frames found in {}",
            frames_dir.display()
        );
        return Ok(());
    };
    let (w, h) = image::image_dimensions(first)?;
    let mut gif = GifWriter::create(output, w, h, fps)?;
    for path in &paths {
        let rgba = image::open(path)?.to_rgba8();
        gif.push_rgba(rgba.as_raw())?;
    }
    gif.finish()?;
    tracing::info!("GIF written to {}", output.display());
    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;
    use tempfile::tempdir;

    fn make_recorder(w: u32, h: u32) -> PhasePortraitRecorder {
        PhasePortraitRecorder::new(w, h, 30, 10, PathBuf::from("test-output.gif"))
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

    #[test]
    fn gif_delay_rounds_and_clamps() {
        assert_eq!(gif_delay_centis(15), 7); // 6.67 cs rounds to 7, not down to 6
        assert_eq!(gif_delay_centis(30), 3);
        assert_eq!(gif_delay_centis(10), 10);
        assert_eq!(gif_delay_centis(120), 2); // browsers slow down anything under 2 cs
        assert_eq!(gif_delay_centis(0), 100);
    }

    #[test]
    fn gif_writer_roundtrip() {
        use image::AnimationDecoder;
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("out.gif");
        let (w, h) = (16u32, 8u32);
        let mut gif = GifWriter::create(&path, w, h, 15).unwrap();
        for i in 0..5u8 {
            let mut frame = vec![0u8; (w * h * 4) as usize];
            for px in frame.chunks_mut(4) {
                px.copy_from_slice(&[i * 50, 255 - i * 50, 128, 255]);
            }
            gif.push_rgba(&frame).unwrap();
        }
        assert!(gif.push_rgba(&[0u8; 3]).is_err());
        assert_eq!(gif.finish().unwrap(), 5);

        let file = std::io::BufReader::new(std::fs::File::open(&path).unwrap());
        let frames = image::codecs::gif::GifDecoder::new(file)
            .unwrap()
            .into_frames()
            .collect_frames()
            .unwrap();
        assert_eq!(frames.len(), 5);
        for f in &frames {
            assert_eq!(f.buffer().dimensions(), (w, h));
            let (num, den) = f.delay().numer_denom_ms();
            assert_eq!(num / den, 70);
        }
        // Solid colours survive quantisation closely.
        let p = frames[2].buffer().get_pixel(3, 3).0;
        assert!((p[0] as i32 - 100).abs() <= 8 && (p[1] as i32 - 155).abs() <= 8);
    }

    #[test]
    fn encode_to_gif_from_png_frames() {
        let dir = tempdir().unwrap();
        for i in 0..3u8 {
            let img = image::RgbaImage::from_pixel(6, 4, Rgba([i * 80, 0, 0, 255]));
            img.save(dir.path().join(format!("frame_{i:05}.png")))
                .unwrap();
        }
        let out = dir.path().join("x.gif");
        encode_to_gif(dir.path(), &out, 30).unwrap();
        use image::AnimationDecoder;
        let file = std::io::BufReader::new(std::fs::File::open(&out).unwrap());
        let n = image::codecs::gif::GifDecoder::new(file)
            .unwrap()
            .into_frames()
            .count();
        assert_eq!(n, 3);
    }

    #[test]
    fn apng_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a").join("out.png");
        let (w, h) = (8u32, 4u32);
        let mut apng = ApngWriter::create(&path, w, h, 30, 3).unwrap();
        for i in 0..3u8 {
            let frame: Vec<u8> = (0..w * h).flat_map(|_| [i * 100, 7, 200, 255]).collect();
            apng.push_rgba(&frame).unwrap();
        }
        assert!(
            apng.push_rgba(&[0u8; 8 * 4 * 4]).is_err(),
            "a 4th frame is rejected"
        );
        assert_eq!(apng.finish().unwrap(), 3);

        let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
        let mut reader = dec.read_info().unwrap();
        let info = reader.info();
        let actl = info.animation_control().expect("animated");
        assert_eq!(actl.num_frames, 3);
        assert_eq!(actl.num_plays, 0);
        let fctl = info.frame_control().unwrap();
        assert_eq!((fctl.delay_num, fctl.delay_den), (1, 30));
        let mut buf = vec![0u8; reader.output_buffer_size().unwrap()];
        for i in 0..3u8 {
            reader.next_frame(&mut buf).unwrap();
            // Full colour survives exactly (no palette quantisation).
            assert_eq!(&buf[..4], &[i * 100, 7, 200, 255]);
        }
    }

    #[test]
    fn apng_too_few_frames_is_error() {
        let dir = tempdir().unwrap();
        let mut apng = ApngWriter::create(&dir.path().join("x.png"), 2, 2, 30, 2).unwrap();
        apng.push_rgba(&[0u8; 16]).unwrap();
        assert!(apng.finish().is_err());
        assert!(ApngWriter::create(&dir.path().join("y.png"), 2, 2, 30, 0).is_err());
    }

    #[test]
    fn ffmpeg_missing_is_a_clear_error() {
        let dir = tempdir().unwrap();
        let err = FfmpegWriter::spawn(
            "definitely-not-ffmpeg-geodesic",
            &dir.path().join("x.mp4"),
            4,
            4,
            30,
        )
        .err()
        .expect("spawn must fail");
        assert!(err.to_string().contains("ffmpeg"), "{err}");
    }

    /// Runs only where ffmpeg is installed (it is optional).
    #[test]
    fn ffmpeg_writes_mp4_when_available() {
        let ok = std::process::Command::new("ffmpeg")
            .arg("-version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            eprintln!("ffmpeg not on PATH; skipping");
            return;
        }
        let dir = tempdir().unwrap();
        let path = dir.path().join("clip.mp4");
        // Odd size on purpose: the writer pads to even.
        let (w, h) = (33u32, 17u32);
        let mut mp4 = FfmpegWriter::spawn("ffmpeg", &path, w, h, 30).unwrap();
        for i in 0..10u8 {
            mp4.push_rgba(&vec![i * 20; (w * h * 4) as usize]).unwrap();
        }
        assert_eq!(mp4.finish().unwrap(), 10);
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 100);
        assert_eq!(&bytes[4..8], b"ftyp");
    }
}
