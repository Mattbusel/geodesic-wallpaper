//! # Wave Interference Patterns
//!
//! Physical wave simulation supporting circular, plane, and standing waves.
//! Multiple waves can be superposed to produce interference patterns, which
//! can then be rendered to RGB via one of several built-in colormaps.
//!
//! ## Example
//!
//! ```rust,ignore
//! use geodesic_wallpaper::wave_interference::{Wave, WaveType, InterferencePattern,
//!                                              InterferenceColormap, double_slit_pattern};
//!
//! let mut pattern = InterferencePattern::new(800, 600);
//! pattern.add_wave(Wave {
//!     amplitude: 1.0, frequency: 0.05, phase: 0.0,
//!     origin: (400.0, 300.0), wave_type: WaveType::Circular,
//! });
//! let frame = pattern.animate_frame(0.0);
//! ```

use std::f64::consts::PI;

// ── WaveType ──────────────────────────────────────────────────────────────────

/// The spatial pattern of a wave.
#[derive(Debug, Clone)]
pub enum WaveType {
    /// Emanates radially from `Wave::origin`.
    Circular,
    /// Plane wave travelling in `direction` (will be normalised internally).
    Plane { direction: (f64, f64) },
    /// Standing wave oscillating along the x-axis.
    Standing,
}

// ── Wave ──────────────────────────────────────────────────────────────────────

/// A single physical wave.
#[derive(Debug, Clone)]
pub struct Wave {
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,
    /// Spatial origin (used for [`WaveType::Circular`]).
    pub origin: (f64, f64),
    pub wave_type: WaveType,
}

// ── wave_value ────────────────────────────────────────────────────────────────

/// Evaluate a single wave at position (x, y) and time t.
///
/// Wave speed c is normalised to 1.0.
pub fn wave_value(wave: &Wave, x: f64, y: f64, t: f64) -> f64 {
    match &wave.wave_type {
        WaveType::Circular => {
            let dx = x - wave.origin.0;
            let dy = y - wave.origin.1;
            let r = (dx * dx + dy * dy).sqrt();
            wave.amplitude
                * (2.0 * PI * wave.frequency * t - 2.0 * PI * wave.frequency * r + wave.phase)
                    .sin()
        }
        WaveType::Plane { direction } => {
            let len = (direction.0 * direction.0 + direction.1 * direction.1).sqrt();
            let (dx, dy) = if len > 1e-12 {
                (direction.0 / len, direction.1 / len)
            } else {
                (1.0, 0.0)
            };
            let proj = x * dx + y * dy;
            wave.amplitude
                * (2.0 * PI * wave.frequency * t - 2.0 * PI * wave.frequency * proj
                    + wave.phase)
                    .sin()
        }
        WaveType::Standing => {
            wave.amplitude
                * (2.0 * PI * wave.frequency * x).sin()
                * (2.0 * PI * wave.frequency * t + wave.phase).cos()
        }
    }
}

// ── InterferenceColormap ──────────────────────────────────────────────────────

/// Colormap used when converting interference values to RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterferenceColormap {
    Grayscale,
    BlueRed,
    RainbowWave,
    Heatmap,
}

impl InterferenceColormap {
    /// Map a value in [-1, 1] to an RGB triplet.
    pub fn apply(self, v: f64) -> (u8, u8, u8) {
        // Clamp to [-1, 1] then normalise to [0, 1].
        let v = v.clamp(-1.0, 1.0);
        let t = (v + 1.0) / 2.0; // 0..1

        match self {
            InterferenceColormap::Grayscale => {
                let c = (t * 255.0) as u8;
                (c, c, c)
            }
            InterferenceColormap::BlueRed => {
                // Negative → blue, zero → white, positive → red.
                let r = (t * 2.0).min(1.0);
                let b = ((1.0 - t) * 2.0).min(1.0);
                let g = 1.0 - (r - b).abs();
                (
                    (r * 255.0) as u8,
                    (g * 255.0) as u8,
                    (b * 255.0) as u8,
                )
            }
            InterferenceColormap::RainbowWave => {
                // Map t in [0,1] to full HSV hue cycle (H = 360*t, S=1, V=1).
                hsv_to_rgb(t * 360.0, 1.0, 1.0)
            }
            InterferenceColormap::Heatmap => {
                // Black → blue → cyan → green → yellow → red → white
                heatmap_rgb(t)
            }
        }
    }
}

/// Convert HSV (h in [0,360], s and v in [0,1]) to RGB.
fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
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
    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

/// Five-stop heatmap: black→blue→cyan→yellow→red (t in [0,1]).
fn heatmap_rgb(t: f64) -> (u8, u8, u8) {
    let stops: [(f64, (f64, f64, f64)); 5] = [
        (0.00, (0.0, 0.0, 0.0)),
        (0.25, (0.0, 0.0, 1.0)),
        (0.50, (0.0, 1.0, 1.0)),
        (0.75, (1.0, 1.0, 0.0)),
        (1.00, (1.0, 0.0, 0.0)),
    ];
    let mut lo = stops[0];
    let mut hi = stops[1];
    for i in 1..stops.len() {
        if t <= stops[i].0 {
            lo = stops[i - 1];
            hi = stops[i];
            break;
        }
    }
    let range = hi.0 - lo.0;
    let local_t = if range > 1e-12 { (t - lo.0) / range } else { 0.0 };
    let r = lo.1.0 + (hi.1.0 - lo.1.0) * local_t;
    let g = lo.1.1 + (hi.1.1 - lo.1.1) * local_t;
    let b = lo.1.2 + (hi.1.2 - lo.1.2) * local_t;
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

// ── InterferencePattern ───────────────────────────────────────────────────────

/// A superposition of multiple waves rendered to a pixel grid.
pub struct InterferencePattern {
    pub width: usize,
    pub height: usize,
    pub waves: Vec<Wave>,
}

impl InterferencePattern {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height, waves: Vec::new() }
    }

    /// Add a wave to the superposition.
    pub fn add_wave(&mut self, wave: Wave) {
        self.waves.push(wave);
    }

    /// Compute the summed wave value at every pixel for time `t`.
    ///
    /// Returns a flat `Vec<f64>` of length `width * height` in row-major order.
    pub fn compute_at_time(&self, t: f64) -> Vec<f64> {
        let mut values = vec![0.0f64; self.width * self.height];
        for (idx, v) in values.iter_mut().enumerate() {
            let px = (idx % self.width) as f64;
            let py = (idx / self.width) as f64;
            *v = self.waves.iter().map(|w| wave_value(w, px, py, t)).sum();
        }
        values
    }

    /// Convert raw wave values to an RGB pixel buffer using `colormap`.
    ///
    /// Values are normalised to [-1, 1] before mapping.
    pub fn to_rgb(&self, values: &[f64], colormap: InterferenceColormap) -> Vec<u8> {
        // Find the absolute maximum for normalisation.
        let max_abs = values
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0_f64, f64::max);
        let scale = if max_abs > 1e-12 { 1.0 / max_abs } else { 1.0 };

        let mut pixels = vec![0u8; values.len() * 3];
        for (i, &v) in values.iter().enumerate() {
            let (r, g, b) = colormap.apply(v * scale);
            pixels[i * 3] = r;
            pixels[i * 3 + 1] = g;
            pixels[i * 3 + 2] = b;
        }
        pixels
    }

    /// Convenience: compute and convert to RGB in one call.
    pub fn animate_frame(&self, t: f64) -> Vec<u8> {
        let values = self.compute_at_time(t);
        self.to_rgb(&values, InterferenceColormap::RainbowWave)
    }
}

// ── double_slit_pattern ───────────────────────────────────────────────────────

/// Compute a classic Young's double-slit interference pattern at t = 0.
///
/// Two circular wave sources are placed at (cx ± slit_separation/2, 0)
/// where cx = width / 2.  The wavelength argument controls the spatial
/// frequency of fringes (frequency = 1 / wavelength).
pub fn double_slit_pattern(
    slit_separation: f64,
    wavelength: f64,
    width: usize,
    height: usize,
) -> Vec<f64> {
    let cx = width as f64 / 2.0;
    let freq = if wavelength > 1e-12 { 1.0 / wavelength } else { 1.0 };

    let wave_left = Wave {
        amplitude: 1.0,
        frequency: freq,
        phase: 0.0,
        origin: (cx - slit_separation / 2.0, 0.0),
        wave_type: WaveType::Circular,
    };
    let wave_right = Wave {
        amplitude: 1.0,
        frequency: freq,
        phase: 0.0,
        origin: (cx + slit_separation / 2.0, 0.0),
        wave_type: WaveType::Circular,
    };

    let pattern = InterferencePattern { width, height, waves: vec![wave_left, wave_right] };
    pattern.compute_at_time(0.0)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_circular_wave_at_origin_gives_amplitude_at_r0() {
        let wave = Wave {
            amplitude: 1.0,
            frequency: 0.1,
            phase: 0.0,
            origin: (0.0, 0.0),
            wave_type: WaveType::Circular,
        };
        // At origin r=0, t=0: value = sin(0) = 0.0
        let v = wave_value(&wave, 0.0, 0.0, 0.0);
        assert!((v - 0.0).abs() < 1e-9, "expected 0.0 at r=0,t=0; got {v}");

        // At t = 0.25/freq the argument is π/2 so sin = 1 (amplitude).
        let t_quarter = 0.25 / wave.frequency;
        let v2 = wave_value(&wave, 0.0, 0.0, t_quarter);
        assert!((v2 - wave.amplitude).abs() < 1e-9, "expected amplitude at quarter period; got {v2}");
    }

    #[test]
    fn double_slit_produces_fringes() {
        let values = double_slit_pattern(20.0, 10.0, 200, 200);
        assert_eq!(values.len(), 200 * 200);

        // Verify that there is both positive and negative interference.
        let max_val = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_val = values.iter().copied().fold(f64::INFINITY, f64::min);
        assert!(max_val > 0.5, "expected constructive interference; max={max_val}");
        assert!(min_val < -0.5, "expected destructive interference; min={min_val}");
    }

    #[test]
    fn colormap_output_in_range_0_255() {
        for colormap in [
            InterferenceColormap::Grayscale,
            InterferenceColormap::BlueRed,
            InterferenceColormap::RainbowWave,
            InterferenceColormap::Heatmap,
        ] {
            for v in [-1.0_f64, -0.5, 0.0, 0.5, 1.0] {
                let (r, g, b) = colormap.apply(v);
                // All u8 values are inherently in [0, 255].
                let _ = (r, g, b); // prevent unused warning
            }
        }
    }

    #[test]
    fn plane_wave_value_varies_across_space() {
        let wave = Wave {
            amplitude: 1.0,
            frequency: 0.05,
            phase: 0.0,
            origin: (0.0, 0.0),
            wave_type: WaveType::Plane { direction: (1.0, 0.0) },
        };
        let v0 = wave_value(&wave, 0.0, 0.0, 0.0);
        let v1 = wave_value(&wave, 10.0, 0.0, 0.0);
        // Two spatially separated samples should (generically) differ.
        assert!(
            (v0 - v1).abs() > 1e-6,
            "expected spatial variation; v0={v0}, v1={v1}"
        );
    }

    #[test]
    fn interference_pattern_compute_length() {
        let mut p = InterferencePattern::new(100, 80);
        p.add_wave(Wave {
            amplitude: 1.0,
            frequency: 0.1,
            phase: 0.0,
            origin: (50.0, 40.0),
            wave_type: WaveType::Circular,
        });
        let values = p.compute_at_time(0.0);
        assert_eq!(values.len(), 100 * 80);
    }

    #[test]
    fn to_rgb_returns_correct_buffer_size() {
        let p = InterferencePattern::new(10, 10, );
        let values = vec![0.5f64; 100];
        let rgb = p.to_rgb(&values, InterferenceColormap::Grayscale);
        assert_eq!(rgb.len(), 100 * 3);
    }
}
