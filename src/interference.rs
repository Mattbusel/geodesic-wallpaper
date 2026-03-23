//! Wave interference patterns and diffraction simulation.

use std::f64::consts::PI;

/// A single wave source.
#[derive(Debug, Clone)]
pub struct WaveSource {
    pub x: f64,
    pub y: f64,
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,
}

/// Computes the wave value from a single source at a point and time.
pub fn wave_at(source: &WaveSource, px: f64, py: f64, time: f64) -> f64 {
    let dx = px - source.x;
    let dy = py - source.y;
    let dist = (dx * dx + dy * dy).sqrt();
    let wavelength = 1.0 / source.frequency; // normalized
    source.amplitude * (2.0 * PI * source.frequency * time - 2.0 * PI * dist / wavelength + source.phase).cos()
}

/// Superposes multiple wave sources at a point.
pub fn superpose(sources: &[WaveSource], px: f64, py: f64, time: f64) -> f64 {
    sources.iter().map(|s| wave_at(s, px, py, time)).sum()
}

/// An interference pattern over a 2D region.
pub struct InterferencePattern {
    pub width: u32,
    pub height: u32,
    pub sources: Vec<WaveSource>,
}

impl InterferencePattern {
    pub fn new(width: u32, height: u32, sources: Vec<WaveSource>) -> Self {
        Self { width, height, sources }
    }

    /// Renders the pattern at a given time as an RGB image.
    pub fn render_at_time(&self, t: f64) -> Vec<u8> {
        let total_amplitude: f64 = self.sources.iter().map(|s| s.amplitude).sum();
        let total_amplitude = if total_amplitude == 0.0 { 1.0 } else { total_amplitude };

        let w = self.width as f64;
        let h = self.height as f64;
        let mut buf = Vec::with_capacity((self.width * self.height * 3) as usize);

        for py in 0..self.height {
            for px in 0..self.width {
                let fx = px as f64 / w;
                let fy = py as f64 / h;
                let val = superpose(&self.sources, fx, fy, t);
                // Normalize to [0, 1]
                let normalized = (val / total_amplitude).clamp(-1.0, 1.0);
                // Map to color: positive -> warm (red/orange), negative -> cool (blue/cyan)
                let (r, g, b) = if normalized >= 0.0 {
                    let intensity = normalized;
                    let r = (255.0 * intensity) as u8;
                    let g = (128.0 * intensity) as u8;
                    let b = 0u8;
                    (r, g, b)
                } else {
                    let intensity = -normalized;
                    let r = 0u8;
                    let g = (200.0 * intensity) as u8;
                    let b = (255.0 * intensity) as u8;
                    (r, g, b)
                };
                buf.push(r);
                buf.push(g);
                buf.push(b);
            }
        }
        buf
    }
}

/// Classifies interference type at a point.
pub fn interference_type_at(sources: &[WaveSource], px: f64, py: f64, t: f64) -> &'static str {
    let value = superpose(sources, px, py, t);
    let max_amplitude: f64 = sources.iter().map(|s| s.amplitude).sum();
    if max_amplitude == 0.0 {
        return "partial";
    }
    if value > 0.7 * max_amplitude {
        "constructive"
    } else if value < -0.7 * max_amplitude {
        "destructive"
    } else {
        "partial"
    }
}

/// Double-slit experiment model.
pub struct DoubleSlit {
    pub slit_separation: f64,
    pub wavelength: f64,
    pub screen_distance: f64,
}

impl DoubleSlit {
    /// Intensity at a given angle from center (in degrees).
    /// I = cos²(π * d * sin(θ) / λ)
    pub fn intensity_at(&self, angle_deg: f64) -> f64 {
        let theta = angle_deg.to_radians();
        let arg = PI * self.slit_separation * theta.sin() / self.wavelength;
        arg.cos().powi(2)
    }

    /// Generates a grayscale intensity pattern across a screen.
    pub fn pattern(&self, screen_width: u32, pixels_per_unit: f64) -> Vec<u8> {
        let center = screen_width as f64 / 2.0;
        (0..screen_width)
            .map(|px| {
                let offset = (px as f64 - center) / pixels_per_unit;
                let angle_rad = (offset / self.screen_distance).atan();
                let angle_deg = angle_rad.to_degrees();
                let intensity = self.intensity_at(angle_deg);
                (intensity * 255.0).clamp(0.0, 255.0) as u8
            })
            .collect()
    }
}

/// Single-slit diffraction model.
pub struct SingleSlit {
    pub slit_width: f64,
    pub wavelength: f64,
}

impl SingleSlit {
    /// Intensity at a given angle (in degrees) using the sinc² formula.
    /// β = π * a * sin(θ) / λ
    /// I = (sin(β)/β)²
    pub fn intensity_at(&self, angle_deg: f64) -> f64 {
        let theta = angle_deg.to_radians();
        let beta = PI * self.slit_width * theta.sin() / self.wavelength;
        if beta.abs() < 1e-10 {
            1.0 // sinc(0) = 1
        } else {
            (beta.sin() / beta).powi(2)
        }
    }
}

/// Renders a two-source interference animation.
pub fn render_two_source_animation(
    sources: [WaveSource; 2],
    width: u32,
    height: u32,
    frames: usize,
) -> Vec<Vec<u8>> {
    let pattern = InterferencePattern::new(width, height, sources.to_vec());
    (0..frames)
        .map(|f| {
            let t = f as f64 / frames as f64;
            pattern.render_at_time(t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn superpose_sums_correctly() {
        let s1 = WaveSource { x: 0.0, y: 0.0, amplitude: 1.0, frequency: 1.0, phase: 0.0 };
        let s2 = WaveSource { x: 0.0, y: 0.0, amplitude: 1.0, frequency: 1.0, phase: 0.0 };
        let combined = superpose(&[s1.clone(), s2.clone()], 0.0, 0.0, 0.0);
        let individual = wave_at(&s1, 0.0, 0.0, 0.0) + wave_at(&s2, 0.0, 0.0, 0.0);
        assert!((combined - individual).abs() < 1e-10);
    }

    #[test]
    fn double_slit_at_zero_degrees_is_maximum() {
        let ds = DoubleSlit {
            slit_separation: 2.0,
            wavelength: 0.5,
            screen_distance: 1.0,
        };
        let intensity = ds.intensity_at(0.0);
        assert!((intensity - 1.0).abs() < 1e-10, "At 0°, intensity should be 1.0");
    }

    #[test]
    fn single_slit_sinc_peak_at_zero() {
        let ss = SingleSlit {
            slit_width: 0.5,
            wavelength: 0.1,
        };
        let intensity_at_zero = ss.intensity_at(0.0);
        let intensity_at_5deg = ss.intensity_at(5.0);
        assert!(
            (intensity_at_zero - 1.0).abs() < 1e-10,
            "Sinc peak at 0° should be 1.0"
        );
        assert!(
            intensity_at_zero >= intensity_at_5deg,
            "Peak should be at 0°"
        );
    }

    #[test]
    fn render_returns_correct_buffer_size() {
        let sources = vec![
            WaveSource { x: 0.3, y: 0.5, amplitude: 1.0, frequency: 2.0, phase: 0.0 },
        ];
        let pattern = InterferencePattern::new(16, 8, sources);
        let buf = pattern.render_at_time(0.0);
        assert_eq!(buf.len(), 16 * 8 * 3);
    }

    #[test]
    fn constructive_destructive_classification() {
        let s = WaveSource { x: 0.5, y: 0.5, amplitude: 1.0, frequency: 1.0, phase: 0.0 };
        // At the source location and t=0, wave_at = amplitude * cos(0) = 1.0 (constructive)
        let itype = interference_type_at(&[s], 0.5, 0.5, 0.0);
        assert_eq!(itype, "constructive");
    }

    #[test]
    fn two_source_animation_frame_count() {
        let s1 = WaveSource { x: 0.3, y: 0.5, amplitude: 1.0, frequency: 1.0, phase: 0.0 };
        let s2 = WaveSource { x: 0.7, y: 0.5, amplitude: 1.0, frequency: 1.0, phase: 0.0 };
        let frames = render_two_source_animation([s1, s2], 8, 8, 5);
        assert_eq!(frames.len(), 5);
        assert_eq!(frames[0].len(), 8 * 8 * 3);
    }
}
