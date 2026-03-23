//! Color palette generator with HSL-to-RGB conversion.
//!
//! Generates palettes using classical color theory relationships:
//! - `Monochromatic` — shades of a single hue
//! - `Complementary` — two opposing hues (180° apart)
//! - `Triadic` — three equidistant hues (120° apart)
//! - `Analogous` — adjacent hues (30° steps)
//! - `Rainbow` — evenly spread hues
//!
//! # CLI integration
//!
//! ```text
//! geodesic-wallpaper --palette triadic:240 --palette-steps 8
//! geodesic-wallpaper --palette rainbow --palette-steps 12
//! ```
//!
//! # Example
//!
//! ```rust
//! use geodesic_wallpaper::palette::{PaletteGenerator, PaletteType};
//!
//! let palette = PaletteGenerator::generate(PaletteType::Triadic(240.0), 6);
//! println!("Generated {} colors", palette.colors.len());
//! for color in &palette.colors {
//!     println!("rgb({}, {}, {})", color[0], color[1], color[2]);
//! }
//! ```

// ── Palette ───────────────────────────────────────────────────────────────────

/// A named collection of RGB colors.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorPalette {
    /// RGB colors, each as `[R, G, B]` with values in 0..=255.
    pub colors: Vec<[u8; 3]>,
    /// Human-readable name of the palette.
    pub name: String,
}

impl ColorPalette {
    /// Create a new named palette.
    pub fn new(name: impl Into<String>, colors: Vec<[u8; 3]>) -> Self {
        Self { name: name.into(), colors }
    }

    /// Convert all colors to CSS hex strings (e.g. `"#4488FF"`).
    pub fn to_hex_strings(&self) -> Vec<String> {
        self.colors
            .iter()
            .map(|[r, g, b]| format!("#{:02X}{:02X}{:02X}", r, g, b))
            .collect()
    }
}

// ── Palette type ──────────────────────────────────────────────────────────────

/// The type of color relationship to use when generating a palette.
///
/// The hue argument is in degrees (0..360).
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteType {
    /// Shades and tints of a single hue.
    Monochromatic(f32),
    /// Two complementary hues 180° apart.
    Complementary(f32),
    /// Three equidistant hues at 120° intervals.
    Triadic(f32),
    /// Adjacent hues in 30° steps on either side of the base hue.
    Analogous(f32),
    /// Full rainbow spread evenly across all hues.
    Rainbow,
}

impl PaletteType {
    /// Parse a palette type from a string like `"triadic:240"` or `"rainbow"`.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_lowercase();
        if s == "rainbow" {
            return Some(PaletteType::Rainbow);
        }
        let (kind, hue) = if let Some((k, h)) = s.split_once(':') {
            let hue: f32 = h.parse().ok()?;
            (k.to_string(), hue)
        } else {
            return None;
        };
        match kind.as_str() {
            "mono" | "monochromatic" => Some(PaletteType::Monochromatic(hue)),
            "complementary" => Some(PaletteType::Complementary(hue)),
            "triadic" => Some(PaletteType::Triadic(hue)),
            "analogous" => Some(PaletteType::Analogous(hue)),
            _ => None,
        }
    }
}

// ── HSL to RGB ────────────────────────────────────────────────────────────────

/// Convert HSL values to an RGB triple.
///
/// - `h`: hue in degrees (0..360)
/// - `s`: saturation (0..=1)
/// - `l`: lightness (0..=1)
///
/// Returns `[R, G, B]` with values in 0..=255.
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return [v, v, v];
    }

    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
    ]
}

fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
    let t = t.rem_euclid(1.0);
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

// ── PaletteGenerator ──────────────────────────────────────────────────────────

/// Generates color palettes from color-theory relationships.
pub struct PaletteGenerator;

impl PaletteGenerator {
    /// Generate a `ColorPalette` with `n_colors` entries.
    ///
    /// - For multi-hue palette types: colors cycle through the hue set, varying lightness.
    /// - For `Monochromatic`: same hue with varying lightness from 0.25 to 0.75.
    pub fn generate(palette_type: PaletteType, n_colors: usize) -> ColorPalette {
        let n = n_colors.max(1);

        match palette_type {
            PaletteType::Monochromatic(hue) => {
                let name = format!("monochromatic:{:.0}", hue);
                let colors = (0..n)
                    .map(|i| {
                        let l = if n == 1 {
                            0.5
                        } else {
                            0.25 + (i as f32 / (n - 1) as f32) * 0.5
                        };
                        hsl_to_rgb(hue, 0.8, l)
                    })
                    .collect();
                ColorPalette::new(name, colors)
            }

            PaletteType::Complementary(hue) => {
                let name = format!("complementary:{:.0}", hue);
                let hues = [hue, hue + 180.0];
                let colors = Self::multi_hue_palette(&hues, n);
                ColorPalette::new(name, colors)
            }

            PaletteType::Triadic(hue) => {
                let name = format!("triadic:{:.0}", hue);
                let hues = [hue, hue + 120.0, hue + 240.0];
                let colors = Self::multi_hue_palette(&hues, n);
                ColorPalette::new(name, colors)
            }

            PaletteType::Analogous(hue) => {
                let name = format!("analogous:{:.0}", hue);
                let hues = [hue - 30.0, hue, hue + 30.0];
                let colors = Self::multi_hue_palette(&hues, n);
                ColorPalette::new(name, colors)
            }

            PaletteType::Rainbow => {
                let name = "rainbow".to_string();
                let colors = (0..n)
                    .map(|i| {
                        let t = i as f32 / n as f32;
                        hsl_to_rgb(t * 360.0, 0.9, 0.55)
                    })
                    .collect();
                ColorPalette::new(name, colors)
            }
        }
    }

    /// Distribute `n` colors across a set of hues, cycling through them and varying lightness.
    fn multi_hue_palette(hues: &[f32], n: usize) -> Vec<[u8; 3]> {
        (0..n)
            .map(|i| {
                let hue = hues[i % hues.len()];
                let l = if n == 1 {
                    0.5
                } else {
                    let cycle_pos = (i / hues.len()) as f32
                        / ((n.saturating_sub(1) / hues.len()).max(1)) as f32;
                    0.35 + cycle_pos * 0.3
                };
                hsl_to_rgb(hue, 0.85, l.clamp(0.25, 0.75))
            })
            .collect()
    }

    /// Parse a CLI-style palette spec and generate a palette.
    ///
    /// `spec` can be e.g. `"triadic:240"`, `"rainbow"`, `"monochromatic:120"`.
    /// Returns `None` if the spec can't be parsed.
    pub fn from_spec(spec: &str, n_colors: usize) -> Option<ColorPalette> {
        PaletteType::parse(spec).map(|pt| Self::generate(pt, n_colors))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── HSL conversion tests ──────────────────────────────────────────────────

    #[test]
    fn test_hsl_red() {
        let rgb = hsl_to_rgb(0.0, 1.0, 0.5);
        assert_eq!(rgb[0], 255);
        assert_eq!(rgb[1], 0);
        assert_eq!(rgb[2], 0);
    }

    #[test]
    fn test_hsl_green() {
        let rgb = hsl_to_rgb(120.0, 1.0, 0.5);
        assert_eq!(rgb[0], 0);
        assert_eq!(rgb[1], 255);
        assert_eq!(rgb[2], 0);
    }

    #[test]
    fn test_hsl_blue() {
        let rgb = hsl_to_rgb(240.0, 1.0, 0.5);
        assert_eq!(rgb[0], 0);
        assert_eq!(rgb[1], 0);
        assert_eq!(rgb[2], 255);
    }

    #[test]
    fn test_hsl_white() {
        let rgb = hsl_to_rgb(0.0, 0.0, 1.0);
        assert_eq!(rgb, [255, 255, 255]);
    }

    #[test]
    fn test_hsl_black() {
        let rgb = hsl_to_rgb(0.0, 0.0, 0.0);
        assert_eq!(rgb, [0, 0, 0]);
    }

    #[test]
    fn test_hsl_grey() {
        let rgb = hsl_to_rgb(180.0, 0.0, 0.5);
        assert_eq!(rgb[0], rgb[1]);
        assert_eq!(rgb[1], rgb[2]);
    }

    #[test]
    fn test_hsl_hue_wraps_360() {
        let a = hsl_to_rgb(0.0, 0.8, 0.5);
        let b = hsl_to_rgb(360.0, 0.8, 0.5);
        assert_eq!(a, b);
    }

    // ── Palette type parsing ──────────────────────────────────────────────────

    #[test]
    fn test_parse_rainbow() {
        assert_eq!(PaletteType::parse("rainbow"), Some(PaletteType::Rainbow));
    }

    #[test]
    fn test_parse_triadic() {
        assert_eq!(PaletteType::parse("triadic:240"), Some(PaletteType::Triadic(240.0)));
    }

    #[test]
    fn test_parse_monochromatic() {
        assert_eq!(PaletteType::parse("monochromatic:120"), Some(PaletteType::Monochromatic(120.0)));
        assert_eq!(PaletteType::parse("mono:60"), Some(PaletteType::Monochromatic(60.0)));
    }

    #[test]
    fn test_parse_complementary() {
        assert_eq!(PaletteType::parse("complementary:30"), Some(PaletteType::Complementary(30.0)));
    }

    #[test]
    fn test_parse_analogous() {
        assert_eq!(PaletteType::parse("analogous:180"), Some(PaletteType::Analogous(180.0)));
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(PaletteType::parse("unknown:100"), None);
        assert_eq!(PaletteType::parse("triadic:notanumber"), None);
    }

    // ── PaletteGenerator tests ────────────────────────────────────────────────

    #[test]
    fn test_rainbow_correct_count() {
        let p = PaletteGenerator::generate(PaletteType::Rainbow, 8);
        assert_eq!(p.colors.len(), 8);
        assert_eq!(p.name, "rainbow");
    }

    #[test]
    fn test_rainbow_colors_all_valid() {
        let p = PaletteGenerator::generate(PaletteType::Rainbow, 12);
        for c in &p.colors {
            // All values must be in 0..=255 (always true for u8, just ensure no panic)
            assert!(c[0] <= 255 && c[1] <= 255 && c[2] <= 255);
        }
    }

    #[test]
    fn test_monochromatic_single_color() {
        let p = PaletteGenerator::generate(PaletteType::Monochromatic(120.0), 1);
        assert_eq!(p.colors.len(), 1);
    }

    #[test]
    fn test_monochromatic_name() {
        let p = PaletteGenerator::generate(PaletteType::Monochromatic(120.0), 4);
        assert!(p.name.contains("mono"), "name: {}", p.name);
    }

    #[test]
    fn test_complementary_two_hue_groups() {
        let p = PaletteGenerator::generate(PaletteType::Complementary(0.0), 4);
        assert_eq!(p.colors.len(), 4);
        // Colors at index 0 and 1 should differ (different hues)
        assert_ne!(p.colors[0], p.colors[1]);
    }

    #[test]
    fn test_triadic_n_colors() {
        let p = PaletteGenerator::generate(PaletteType::Triadic(0.0), 9);
        assert_eq!(p.colors.len(), 9);
    }

    #[test]
    fn test_analogous_name() {
        let p = PaletteGenerator::generate(PaletteType::Analogous(60.0), 6);
        assert!(p.name.contains("analogous"));
    }

    #[test]
    fn test_hex_strings_format() {
        let p = PaletteGenerator::generate(PaletteType::Rainbow, 3);
        let hexes = p.to_hex_strings();
        assert_eq!(hexes.len(), 3);
        for h in &hexes {
            assert_eq!(h.len(), 7, "hex string should be #RRGGBB: {}", h);
            assert!(h.starts_with('#'));
        }
    }

    #[test]
    fn test_from_spec_triadic() {
        let p = PaletteGenerator::from_spec("triadic:120", 6).unwrap();
        assert_eq!(p.colors.len(), 6);
    }

    #[test]
    fn test_from_spec_invalid_returns_none() {
        assert!(PaletteGenerator::from_spec("unknown:90", 4).is_none());
    }

    #[test]
    fn test_min_one_color_with_zero_request() {
        let p = PaletteGenerator::generate(PaletteType::Rainbow, 0);
        assert_eq!(p.colors.len(), 1, "should produce at least 1 color");
    }

    #[test]
    fn test_color_palette_new() {
        let p = ColorPalette::new("test", vec![[255, 0, 0]]);
        assert_eq!(p.name, "test");
        assert_eq!(p.colors[0], [255, 0, 0]);
    }

    #[test]
    fn test_hex_string_known_red() {
        let p = ColorPalette::new("red", vec![[255, 0, 0]]);
        assert_eq!(p.to_hex_strings()[0], "#FF0000");
    }
}
