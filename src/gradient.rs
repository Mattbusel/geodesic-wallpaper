//! Gradient texture generator for wallpaper patterns.
//!
//! Provides smooth linear color gradients mapped over a wallpaper pattern
//! via a function that returns a float in [0, 1].
//!
//! # Example
//!
//! ```rust
//! use geodesic_wallpaper::gradient::{Gradient, GradientPreset, GradientTexture};
//!
//! let gradient = GradientPreset::Sunset.into_gradient();
//! let sample = gradient.sample(0.5);
//! println!("rgb({}, {}, {})", sample[0], sample[1], sample[2]);
//!
//! let pixels = GradientTexture::generate(
//!     4, 4,
//!     |x, y| (x as f32 + y as f32) / 6.0,
//!     &gradient,
//! );
//! assert_eq!(pixels.len(), 16);
//! ```

// ── GradientStop ──────────────────────────────────────────────────────────────

/// A single stop in a color gradient.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    /// Position in [0, 1].
    pub position: f32,
    /// RGB color at this position.
    pub color: [u8; 3],
}

impl GradientStop {
    pub fn new(position: f32, color: [u8; 3]) -> Self {
        Self { position: position.clamp(0.0, 1.0), color }
    }
}

// ── Gradient ──────────────────────────────────────────────────────────────────

/// A linear gradient defined by a sorted list of color stops.
#[derive(Debug, Clone)]
pub struct Gradient {
    /// Color stops, sorted by position.
    pub stops: Vec<GradientStop>,
}

impl Gradient {
    /// Create a gradient from a list of stops. Stops are sorted by position.
    pub fn new(mut stops: Vec<GradientStop>) -> Self {
        stops.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));
        Self { stops }
    }

    /// Sample the gradient at position `t` in [0, 1].
    ///
    /// Returns black if there are no stops, the first color if t ≤ first stop,
    /// the last color if t ≥ last stop, or a linearly-interpolated RGB value
    /// between the two surrounding stops.
    pub fn sample(&self, t: f32) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);

        if self.stops.is_empty() {
            return [0, 0, 0];
        }
        if self.stops.len() == 1 {
            return self.stops[0].color;
        }

        // Before first stop
        if t <= self.stops[0].position {
            return self.stops[0].color;
        }
        // After last stop
        if t >= self.stops[self.stops.len() - 1].position {
            return self.stops[self.stops.len() - 1].color;
        }

        // Find surrounding stops
        let right = self
            .stops
            .iter()
            .position(|s| s.position > t)
            .unwrap_or(self.stops.len() - 1);
        let left = right.saturating_sub(1);

        let s0 = &self.stops[left];
        let s1 = &self.stops[right];

        let span = s1.position - s0.position;
        let alpha = if span < 1e-9 {
            0.0_f32
        } else {
            (t - s0.position) / span
        };

        Self::lerp_rgb(s0.color, s1.color, alpha)
    }

    /// Linear interpolation between two RGB colors.
    fn lerp_rgb(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
        [
            Self::lerp_u8(a[0], b[0], t),
            Self::lerp_u8(a[1], b[1], t),
            Self::lerp_u8(a[2], b[2], t),
        ]
    }

    fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
        let a = a as f32;
        let b = b as f32;
        (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
    }
}

// ── GradientPreset ────────────────────────────────────────────────────────────

/// Built-in gradient presets.
#[derive(Debug, Clone, PartialEq)]
pub enum GradientPreset {
    Sunset,
    Ocean,
    Forest,
    Plasma,
    Greyscale,
    Custom(Vec<GradientStop>),
}

impl GradientPreset {
    /// Convert this preset to a `Gradient`.
    pub fn into_gradient(self) -> Gradient {
        match self {
            GradientPreset::Sunset => Gradient::new(vec![
                GradientStop::new(0.0, [10, 10, 40]),
                GradientStop::new(0.25, [120, 20, 80]),
                GradientStop::new(0.5, [220, 80, 30]),
                GradientStop::new(0.75, [255, 160, 50]),
                GradientStop::new(1.0, [255, 220, 150]),
            ]),
            GradientPreset::Ocean => Gradient::new(vec![
                GradientStop::new(0.0, [0, 10, 60]),
                GradientStop::new(0.3, [0, 60, 120]),
                GradientStop::new(0.6, [0, 120, 180]),
                GradientStop::new(0.85, [100, 200, 220]),
                GradientStop::new(1.0, [200, 240, 255]),
            ]),
            GradientPreset::Forest => Gradient::new(vec![
                GradientStop::new(0.0, [10, 30, 10]),
                GradientStop::new(0.3, [20, 80, 20]),
                GradientStop::new(0.6, [50, 140, 50]),
                GradientStop::new(0.85, [120, 200, 80]),
                GradientStop::new(1.0, [200, 230, 150]),
            ]),
            GradientPreset::Plasma => Gradient::new(vec![
                GradientStop::new(0.0, [13, 8, 135]),
                GradientStop::new(0.25, [126, 3, 168]),
                GradientStop::new(0.5, [204, 71, 120]),
                GradientStop::new(0.75, [248, 149, 64]),
                GradientStop::new(1.0, [240, 249, 33]),
            ]),
            GradientPreset::Greyscale => Gradient::new(vec![
                GradientStop::new(0.0, [0, 0, 0]),
                GradientStop::new(1.0, [255, 255, 255]),
            ]),
            GradientPreset::Custom(stops) => Gradient::new(stops),
        }
    }

    /// Parse a preset name string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sunset" => Some(GradientPreset::Sunset),
            "ocean" => Some(GradientPreset::Ocean),
            "forest" => Some(GradientPreset::Forest),
            "plasma" => Some(GradientPreset::Plasma),
            "greyscale" | "grayscale" | "grey" | "gray" => Some(GradientPreset::Greyscale),
            _ => None,
        }
    }
}

// ── GradientTexture ───────────────────────────────────────────────────────────

/// Generates a flat pixel buffer from a pattern function and a gradient.
pub struct GradientTexture;

impl GradientTexture {
    /// Generate a `width × height` pixel buffer.
    ///
    /// `pattern_fn(x, y)` returns a value in [0, 1] which is mapped through
    /// `gradient` to produce an RGB pixel. The result is a `Vec<[u8;3]>` of
    /// length `width * height` in row-major (y × x) order.
    pub fn generate(
        width: u32,
        height: u32,
        pattern_fn: impl Fn(u32, u32) -> f32,
        gradient: &Gradient,
    ) -> Vec<[u8; 3]> {
        let mut pixels = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let t = pattern_fn(x, y).clamp(0.0, 1.0);
                pixels.push(gradient.sample(t));
            }
        }
        pixels
    }

    /// Generate and convert to a flat `Vec<u8>` in RGB byte order.
    pub fn generate_bytes(
        width: u32,
        height: u32,
        pattern_fn: impl Fn(u32, u32) -> f32,
        gradient: &Gradient,
    ) -> Vec<u8> {
        Self::generate(width, height, pattern_fn, gradient)
            .into_iter()
            .flat_map(|[r, g, b]| [r, g, b])
            .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── GradientStop ──────────────────────────────────────────────────────

    #[test]
    fn test_gradient_stop_clamps_position() {
        let s = GradientStop::new(-0.5, [100, 100, 100]);
        assert_eq!(s.position, 0.0);
        let s2 = GradientStop::new(1.5, [100, 100, 100]);
        assert_eq!(s2.position, 1.0);
    }

    // ── Gradient::sample ──────────────────────────────────────────────────

    #[test]
    fn test_sample_empty_gradient() {
        let g = Gradient::new(vec![]);
        assert_eq!(g.sample(0.5), [0, 0, 0]);
    }

    #[test]
    fn test_sample_single_stop() {
        let g = Gradient::new(vec![GradientStop::new(0.5, [128, 64, 32])]);
        assert_eq!(g.sample(0.0), [128, 64, 32]);
        assert_eq!(g.sample(0.5), [128, 64, 32]);
        assert_eq!(g.sample(1.0), [128, 64, 32]);
    }

    #[test]
    fn test_sample_at_zero_returns_first_color() {
        let g = Gradient::new(vec![
            GradientStop::new(0.0, [0, 0, 0]),
            GradientStop::new(1.0, [255, 255, 255]),
        ]);
        assert_eq!(g.sample(0.0), [0, 0, 0]);
    }

    #[test]
    fn test_sample_at_one_returns_last_color() {
        let g = Gradient::new(vec![
            GradientStop::new(0.0, [0, 0, 0]),
            GradientStop::new(1.0, [255, 255, 255]),
        ]);
        assert_eq!(g.sample(1.0), [255, 255, 255]);
    }

    #[test]
    fn test_sample_midpoint_two_stops() {
        let g = Gradient::new(vec![
            GradientStop::new(0.0, [0, 0, 0]),
            GradientStop::new(1.0, [200, 100, 50]),
        ]);
        let mid = g.sample(0.5);
        assert!((mid[0] as i32 - 100).abs() <= 1, "R midpoint: {}", mid[0]);
        assert!((mid[1] as i32 - 50).abs() <= 1, "G midpoint: {}", mid[1]);
        assert!((mid[2] as i32 - 25).abs() <= 1, "B midpoint: {}", mid[2]);
    }

    #[test]
    fn test_sample_before_first_stop() {
        let g = Gradient::new(vec![
            GradientStop::new(0.3, [10, 20, 30]),
            GradientStop::new(1.0, [200, 200, 200]),
        ]);
        assert_eq!(g.sample(0.0), [10, 20, 30]);
    }

    #[test]
    fn test_sample_after_last_stop() {
        let g = Gradient::new(vec![
            GradientStop::new(0.0, [0, 0, 0]),
            GradientStop::new(0.7, [180, 90, 45]),
        ]);
        assert_eq!(g.sample(1.0), [180, 90, 45]);
    }

    #[test]
    fn test_sample_monotone_brightness() {
        let g = GradientPreset::Greyscale.into_gradient();
        let v0 = g.sample(0.0)[0];
        let v5 = g.sample(0.5)[0];
        let v1 = g.sample(1.0)[0];
        assert!(v0 < v5, "greyscale should brighten from 0 to 1");
        assert!(v5 < v1, "greyscale should brighten from 0 to 1");
    }

    #[test]
    fn test_sample_stops_sorted_on_construction() {
        let g = Gradient::new(vec![
            GradientStop::new(1.0, [255, 255, 255]),
            GradientStop::new(0.0, [0, 0, 0]),
        ]);
        assert_eq!(g.sample(0.0), [0, 0, 0]);
        assert_eq!(g.sample(1.0), [255, 255, 255]);
    }

    // ── GradientPreset ────────────────────────────────────────────────────

    #[test]
    fn test_preset_from_str_sunset() {
        assert_eq!(GradientPreset::from_str("sunset"), Some(GradientPreset::Sunset));
        assert_eq!(GradientPreset::from_str("Sunset"), Some(GradientPreset::Sunset));
    }

    #[test]
    fn test_preset_from_str_ocean() {
        assert!(GradientPreset::from_str("ocean").is_some());
    }

    #[test]
    fn test_preset_from_str_forest() {
        assert!(GradientPreset::from_str("forest").is_some());
    }

    #[test]
    fn test_preset_from_str_plasma() {
        assert!(GradientPreset::from_str("plasma").is_some());
    }

    #[test]
    fn test_preset_from_str_greyscale_aliases() {
        assert!(GradientPreset::from_str("greyscale").is_some());
        assert!(GradientPreset::from_str("grayscale").is_some());
        assert!(GradientPreset::from_str("grey").is_some());
        assert!(GradientPreset::from_str("gray").is_some());
    }

    #[test]
    fn test_preset_from_str_unknown() {
        assert!(GradientPreset::from_str("unknown_preset").is_none());
    }

    #[test]
    fn test_all_presets_produce_valid_gradients() {
        for preset in [
            GradientPreset::Sunset,
            GradientPreset::Ocean,
            GradientPreset::Forest,
            GradientPreset::Plasma,
            GradientPreset::Greyscale,
        ] {
            let g = preset.into_gradient();
            assert!(!g.stops.is_empty());
            // Sample at 0, 0.5, 1 — all must produce valid RGB
            for t in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                let c = g.sample(t);
                // Just verify they're u8 values (always true by type, but ensure no panic)
                let _ = c;
            }
        }
    }

    // ── GradientTexture ───────────────────────────────────────────────────

    #[test]
    fn test_generate_correct_pixel_count() {
        let g = GradientPreset::Greyscale.into_gradient();
        let pixels = GradientTexture::generate(4, 4, |_, _| 0.5, &g);
        assert_eq!(pixels.len(), 16);
    }

    #[test]
    fn test_generate_2x2() {
        let g = GradientPreset::Greyscale.into_gradient();
        let pixels = GradientTexture::generate(2, 2, |_, _| 0.5, &g);
        assert_eq!(pixels.len(), 4);
        for p in &pixels {
            assert_eq!(*p, pixels[0], "uniform pattern should give uniform color");
        }
    }

    #[test]
    fn test_generate_pattern_fn_applied() {
        let g = Gradient::new(vec![
            GradientStop::new(0.0, [0, 0, 0]),
            GradientStop::new(1.0, [255, 255, 255]),
        ]);
        // pattern_fn always returns 0 → all pixels should be black
        let pixels = GradientTexture::generate(3, 3, |_, _| 0.0, &g);
        for p in &pixels {
            assert_eq!(p[0], 0);
        }
        // pattern_fn always returns 1 → all pixels should be white
        let pixels = GradientTexture::generate(3, 3, |_, _| 1.0, &g);
        for p in &pixels {
            assert_eq!(p[0], 255);
        }
    }

    #[test]
    fn test_generate_zero_dimensions() {
        let g = GradientPreset::Greyscale.into_gradient();
        let pixels = GradientTexture::generate(0, 0, |_, _| 0.5, &g);
        assert!(pixels.is_empty());
    }

    #[test]
    fn test_generate_bytes_length() {
        let g = GradientPreset::Ocean.into_gradient();
        let bytes = GradientTexture::generate_bytes(4, 4, |x, y| (x + y) as f32 / 6.0, &g);
        assert_eq!(bytes.len(), 4 * 4 * 3);
    }
}
