//! Audio-reactive mode — modulate surface parameters from system audio via WASAPI.
//!
//! Captures the Windows default audio loopback device using WASAPI, computes a
//! short-time FFT, and extracts three frequency bands:
//!
//! | Band  | Frequency range | Maps to |
//! |-------|----------------|---------|
//! | Bass  | 20 – 250 Hz    | Surface curvature (e.g. torus major radius, noise amplitude) |
//! | Mids  | 250 – 4000 Hz  | Trail length |
//! | Highs | 4000 – 20000 Hz| Particle speed |
//!
//! The band energies are smoothed with an exponential moving average to avoid
//! jitter.
//!
//! # Windows WASAPI
//!
//! WASAPI loopback requires no special driver installation and captures the
//! mixed system audio at the device's native sample rate.  We use the `cpal`
//! crate's WASAPI backend because it exposes loopback capture on Windows
//! via [`cpal::platform::Device::default_output_device`].
//!
//! # Fallback
//!
//! When audio capture fails (e.g. no audio device, permissions) the module
//! falls back to returning flat band energies (0.5) so the rest of the app
//! continues to function.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

// ── Band energies ─────────────────────────────────────────────────────────────

/// Smoothed audio band energies in [0, 1].
#[derive(Debug, Clone, Copy, Default)]
pub struct BandEnergies {
    /// Bass energy (20–250 Hz) → surface curvature.
    pub bass: f32,
    /// Mid energy (250–4000 Hz) → trail length.
    pub mids: f32,
    /// High energy (4000–20 000 Hz) → particle speed.
    pub highs: f32,
}

impl BandEnergies {
    /// Map bass energy to a surface curvature multiplier in `[0.5, 3.0]`.
    pub fn curvature(&self) -> f32 {
        0.5 + self.bass * 2.5
    }

    /// Map mids energy to a trail length in frames `[30, 600]`.
    pub fn trail_length_frames(&self) -> usize {
        (30.0 + self.mids * 570.0) as usize
    }

    /// Map highs energy to a particle speed multiplier in `[0.5, 4.0]`.
    pub fn particle_speed(&self) -> f32 {
        0.5 + self.highs * 3.5
    }
}

// ── FFT band splitter ─────────────────────────────────────────────────────────

/// Splits an FFT magnitude spectrum into bass / mids / highs.
pub struct BandSplitter {
    pub sample_rate: f32,
    pub fft_size: usize,
}

impl BandSplitter {
    pub fn new(sample_rate: f32, fft_size: usize) -> Self {
        Self { sample_rate, fft_size: fft_size.max(64) }
    }

    /// Frequency in Hz at bin `k`.
    fn bin_hz(&self, k: usize) -> f32 {
        k as f32 * self.sample_rate / self.fft_size as f32
    }

    /// Sum the RMS of magnitudes in the range `[lo_hz, hi_hz)`.
    fn band_rms(&self, magnitudes: &[f32], lo_hz: f32, hi_hz: f32) -> f32 {
        let nyquist = self.fft_size / 2;
        let lo_bin = ((lo_hz / self.sample_rate * self.fft_size as f32) as usize).min(nyquist);
        let hi_bin = ((hi_hz / self.sample_rate * self.fft_size as f32) as usize).min(nyquist);
        if hi_bin <= lo_bin {
            return 0.0;
        }
        let sum_sq: f32 = magnitudes[lo_bin..hi_bin]
            .iter()
            .map(|&m| m * m)
            .sum();
        let rms = (sum_sq / (hi_bin - lo_bin) as f32).sqrt();
        rms
    }

    /// Compute raw band energies from a magnitude spectrum.
    pub fn split(&self, magnitudes: &[f32]) -> BandEnergies {
        BandEnergies {
            bass: self.band_rms(magnitudes, 20.0, 250.0),
            mids: self.band_rms(magnitudes, 250.0, 4000.0),
            highs: self.band_rms(magnitudes, 4000.0, 20_000.0),
        }
    }
}

// ── Exponential smoother ──────────────────────────────────────────────────────

/// Per-channel EMA smoother for band energies.
pub struct EnergySmoother {
    pub alpha: f32,
    smoothed: BandEnergies,
}

impl EnergySmoother {
    /// Create a smoother with the given EMA coefficient (0 < alpha < 1).
    /// Smaller values = longer memory = more smoothing.
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha: alpha.clamp(1e-4, 1.0),
            smoothed: BandEnergies::default(),
        }
    }

    /// Feed new raw energies and return the smoothed output.
    pub fn update(&mut self, raw: BandEnergies) -> BandEnergies {
        let a = self.alpha;
        self.smoothed.bass  = (1.0 - a) * self.smoothed.bass  + a * raw.bass;
        self.smoothed.mids  = (1.0 - a) * self.smoothed.mids  + a * raw.mids;
        self.smoothed.highs = (1.0 - a) * self.smoothed.highs + a * raw.highs;
        self.smoothed
    }

    pub fn current(&self) -> BandEnergies {
        self.smoothed
    }
}

// ── Shared state ──────────────────────────────────────────────────────────────

/// Thread-safe container for the latest band energies.
pub type SharedEnergies = Arc<Mutex<BandEnergies>>;

/// Create a new shared energies container initialised to 0.5 (neutral).
pub fn shared_energies() -> SharedEnergies {
    Arc::new(Mutex::new(BandEnergies {
        bass: 0.5,
        mids: 0.5,
        highs: 0.5,
    }))
}

// ── Audio capture ─────────────────────────────────────────────────────────────

/// Configuration for the audio capture thread.
#[derive(Debug, Clone)]
pub struct AudioCaptureConfig {
    /// FFT window size (power of 2 recommended).
    pub fft_size: usize,
    /// EMA coefficient for band smoothing (0.05 = gentle, 0.3 = responsive).
    pub smooth_alpha: f32,
    /// Normalisation factor: raw RMS is divided by this before storing.
    pub normalization: f32,
}

impl Default for AudioCaptureConfig {
    fn default() -> Self {
        Self {
            fft_size: 1024,
            smooth_alpha: 0.1,
            normalization: 0.3,
        }
    }
}

/// Runs the WASAPI loopback capture loop in a background thread.
///
/// Captured samples are windowed (Hann), FFT'd, split into bands, smoothed,
/// and written to `energies`.
///
/// Returns `Ok(())` immediately after spawning the background thread.  The
/// thread stops when the `Arc<Mutex<BandEnergies>>` is dropped by all holders.
///
/// # Errors
///
/// Returns an error if no audio input/loopback device is available.
/// In that case the `energies` value is left at its initial value.
pub fn start_capture(
    config: AudioCaptureConfig,
    energies: SharedEnergies,
) -> Result<(), String> {
    // We use rustfft for the FFT computation and drive sample acquisition from
    // a ring buffer filled by a cpal stream.

    let fft_size = config.fft_size;

    // Attempt to open the default output device for loopback capture.
    // On Windows, cpal exposes the WASAPI loopback via the default host.
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();

    // On Windows, use the default output device for loopback.
    // On other platforms, fall back to the default input device.
    #[cfg(target_os = "windows")]
    let device = host
        .default_output_device()
        .ok_or_else(|| "no default output device for WASAPI loopback".to_owned())?;

    #[cfg(not(target_os = "windows"))]
    let device = host
        .default_input_device()
        .ok_or_else(|| "no default input device".to_owned())?;

    let supported_cfg = device
        .default_output_config()
        .map_err(|e| format!("output config error: {e}"))?;
    let sample_rate = supported_cfg.sample_rate().0 as f32;
    let channels = supported_cfg.channels() as usize;

    // Ring buffer shared between audio callback and FFT thread.
    let ring: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::with_capacity(fft_size * 4)));
    let ring_cb = Arc::clone(&ring);

    // Build Hann window.
    let hann: Vec<f32> = (0..fft_size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos())
        })
        .collect();

    // Build FFT planner.
    use rustfft::FftPlanner;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let energies_thread = Arc::clone(&energies);
    let splitter = BandSplitter::new(sample_rate, fft_size);
    let mut smoother = EnergySmoother::new(config.smooth_alpha);
    let norm = config.normalization;

    // Audio callback: mix channels to mono and push to ring buffer.
    let stream = device
        .build_input_stream(
            &supported_cfg.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut ring) = ring_cb.lock() {
                    // Mix to mono.
                    for frame in data.chunks(channels) {
                        let mono = frame.iter().sum::<f32>() / channels as f32;
                        ring.push(mono);
                    }
                }
            },
            |err| log::warn!("[audio-reactive] stream error: {err}"),
            None,
        )
        .map_err(|e| format!("build_input_stream error: {e}"))?;

    stream.play().map_err(|e| format!("stream.play error: {e}"))?;

    // FFT processing thread.
    std::thread::Builder::new()
        .name("audio-reactive-fft".into())
        .spawn(move || {
            // Keep stream alive inside this thread.
            let _stream = stream;
            let mut window_buf = vec![rustfft::num_complex::Complex32::new(0.0, 0.0); fft_size];

            loop {
                // Wait until we have enough samples.
                let samples = {
                    let Ok(mut ring) = ring.lock() else { break };
                    if ring.len() < fft_size {
                        drop(ring);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                        continue;
                    }
                    let s: Vec<f32> = ring.drain(..fft_size).collect();
                    s
                };

                // Apply Hann window and copy to complex buffer.
                for (i, &s) in samples.iter().enumerate() {
                    window_buf[i] =
                        rustfft::num_complex::Complex32::new(s * hann[i], 0.0);
                }

                fft.process(&mut window_buf);

                // Compute magnitudes (first half).
                let magnitudes: Vec<f32> = window_buf[..fft_size / 2]
                    .iter()
                    .map(|c| (c.re * c.re + c.im * c.im).sqrt() / fft_size as f32)
                    .collect();

                let raw = splitter.split(&magnitudes);
                let mut normalised = raw;
                normalised.bass = (raw.bass / norm).clamp(0.0, 1.0);
                normalised.mids = (raw.mids / norm).clamp(0.0, 1.0);
                normalised.highs = (raw.highs / norm).clamp(0.0, 1.0);

                let smoothed = smoother.update(normalised);

                if let Ok(mut e) = energies_thread.lock() {
                    *e = smoothed;
                } else {
                    break; // energies dropped → exit
                }
            }
            log::info!("[audio-reactive] FFT thread exiting");
        })
        .expect("audio-reactive FFT thread");

    log::info!("[audio-reactive] capture started (sample_rate={sample_rate}, fft={fft_size})");
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_band_splitter_frequencies() {
        let splitter = BandSplitter::new(44100.0, 1024);
        // Flat spectrum at unit magnitude.
        let mags = vec![1.0f32; 512];
        let bands = splitter.split(&mags);
        assert!(bands.bass > 0.0, "bass should be non-zero for flat spectrum");
        assert!(bands.mids > 0.0, "mids should be non-zero for flat spectrum");
    }

    #[test]
    fn test_energy_smoother_converges() {
        let mut smoother = EnergySmoother::new(0.5);
        for _ in 0..20 {
            smoother.update(BandEnergies { bass: 1.0, mids: 1.0, highs: 1.0 });
        }
        let c = smoother.current();
        assert!(c.bass > 0.99, "smoother should converge to 1.0: {}", c.bass);
    }

    #[test]
    fn test_band_energies_curvature_range() {
        let e = BandEnergies { bass: 0.0, mids: 0.5, highs: 1.0 };
        assert!((e.curvature() - 0.5).abs() < 1e-4);
        let e2 = BandEnergies { bass: 1.0, mids: 0.0, highs: 0.0 };
        assert!((e2.curvature() - 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_band_energies_trail_length_range() {
        let e_min = BandEnergies { bass: 0.0, mids: 0.0, highs: 0.0 };
        let e_max = BandEnergies { bass: 0.0, mids: 1.0, highs: 0.0 };
        assert_eq!(e_min.trail_length_frames(), 30);
        assert_eq!(e_max.trail_length_frames(), 600);
    }

    #[test]
    fn test_band_energies_particle_speed_range() {
        let e_min = BandEnergies { bass: 0.0, mids: 0.0, highs: 0.0 };
        let e_max = BandEnergies { bass: 0.0, mids: 0.0, highs: 1.0 };
        assert!((e_min.particle_speed() - 0.5).abs() < 1e-4);
        assert!((e_max.particle_speed() - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_shared_energies_default() {
        let e = shared_energies();
        let v = *e.lock().unwrap();
        assert!((v.bass - 0.5).abs() < 1e-4);
    }
}
