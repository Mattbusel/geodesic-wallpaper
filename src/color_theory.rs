//! Color space conversions and harmony-based palette generation.
//!
//! Supports RGB ↔ HSV ↔ HSL ↔ CIELAB conversions (D65 illuminant),
//! WCAG contrast ratio, and multiple color harmony rules.

// ---------------------------------------------------------------------------
// Color structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    /// Hue in [0, 360).
    pub h: f64,
    /// Saturation in [0, 1].
    pub s: f64,
    /// Value in [0, 1].
    pub v: f64,
}

impl Hsv {
    pub fn new(h: f64, s: f64, v: f64) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            s: s.clamp(0.0, 1.0),
            v: v.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

impl Hsl {
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            s: s.clamp(0.0, 1.0),
            l: l.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f64,
    pub a: f64,
    pub b: f64,
}

// ---------------------------------------------------------------------------
// ColorConversions
// ---------------------------------------------------------------------------

pub struct ColorConversions;

impl ColorConversions {
    // -----------------------------------------------------------------------
    // RGB ↔ HSV
    // -----------------------------------------------------------------------

    pub fn rgb_to_hsv(rgb: Rgb) -> Hsv {
        let r = rgb.r as f64 / 255.0;
        let g = rgb.g as f64 / 255.0;
        let b = rgb.b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max < 1e-9 { 0.0 } else { delta / max };

        let h = if delta < 1e-9 {
            0.0
        } else if (max - r).abs() < 1e-9 {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if (max - g).abs() < 1e-9 {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        Hsv::new(h, s, v)
    }

    pub fn hsv_to_rgb(hsv: Hsv) -> Rgb {
        if hsv.s < 1e-9 {
            let v = (hsv.v * 255.0).round() as u8;
            return Rgb::new(v, v, v);
        }
        let h = hsv.h / 60.0;
        let i = h.floor() as u32 % 6;
        let f = h - h.floor();
        let p = hsv.v * (1.0 - hsv.s);
        let q = hsv.v * (1.0 - f * hsv.s);
        let t = hsv.v * (1.0 - (1.0 - f) * hsv.s);

        let (r, g, b) = match i {
            0 => (hsv.v, t, p),
            1 => (q, hsv.v, p),
            2 => (p, hsv.v, t),
            3 => (p, q, hsv.v),
            4 => (t, p, hsv.v),
            _ => (hsv.v, p, q),
        };
        Rgb::new(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        )
    }

    // -----------------------------------------------------------------------
    // RGB ↔ HSL
    // -----------------------------------------------------------------------

    pub fn rgb_to_hsl(rgb: Rgb) -> Hsl {
        let r = rgb.r as f64 / 255.0;
        let g = rgb.g as f64 / 255.0;
        let b = rgb.b as f64 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let l = (max + min) / 2.0;

        let s = if delta < 1e-9 {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };

        let h = if delta < 1e-9 {
            0.0
        } else if (max - r).abs() < 1e-9 {
            60.0 * (((g - b) / delta).rem_euclid(6.0))
        } else if (max - g).abs() < 1e-9 {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        Hsl::new(h, s, l)
    }

    pub fn hsl_to_rgb(hsl: Hsl) -> Rgb {
        if hsl.s < 1e-9 {
            let v = (hsl.l * 255.0).round() as u8;
            return Rgb::new(v, v, v);
        }
        let c = (1.0 - (2.0 * hsl.l - 1.0).abs()) * hsl.s;
        let x = c * (1.0 - ((hsl.h / 60.0) % 2.0 - 1.0).abs());
        let m = hsl.l - c / 2.0;

        let (r1, g1, b1) = match (hsl.h / 60.0).floor() as u32 % 6 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Rgb::new(
            ((r1 + m) * 255.0).round() as u8,
            ((g1 + m) * 255.0).round() as u8,
            ((b1 + m) * 255.0).round() as u8,
        )
    }

    // -----------------------------------------------------------------------
    // RGB ↔ CIELAB (D65 illuminant)
    // -----------------------------------------------------------------------

    pub fn rgb_to_lab(rgb: Rgb) -> Lab {
        let (x, y, z) = Self::rgb_to_xyz(rgb);
        // D65 reference white.
        let xn = 0.95047;
        let yn = 1.00000;
        let zn = 1.08883;

        let fx = Self::f_lab(x / xn);
        let fy = Self::f_lab(y / yn);
        let fz = Self::f_lab(z / zn);

        Lab {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
        }
    }

    pub fn lab_to_rgb(lab: Lab) -> Rgb {
        let fy = (lab.l + 16.0) / 116.0;
        let fx = lab.a / 500.0 + fy;
        let fz = fy - lab.b / 200.0;

        let xn = 0.95047;
        let yn = 1.00000;
        let zn = 1.08883;

        let x = xn * Self::f_lab_inv(fx);
        let y = yn * Self::f_lab_inv(fy);
        let z = zn * Self::f_lab_inv(fz);

        Self::xyz_to_rgb(x, y, z)
    }

    /// Delta-E CIE76 colour distance.
    pub fn color_distance(a: &Lab, b: &Lab) -> f64 {
        let dl = a.l - b.l;
        let da = a.a - b.a;
        let db = a.b - b.b;
        (dl * dl + da * da + db * db).sqrt()
    }

    /// WCAG relative luminance.
    pub fn luminance(rgb: &Rgb) -> f64 {
        fn linearise(c: u8) -> f64 {
            let v = c as f64 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * linearise(rgb.r) + 0.7152 * linearise(rgb.g) + 0.0722 * linearise(rgb.b)
    }

    /// WCAG contrast ratio between two colours.
    pub fn contrast_ratio(a: &Rgb, b: &Rgb) -> f64 {
        let la = Self::luminance(a);
        let lb = Self::luminance(b);
        let lighter = la.max(lb);
        let darker = la.min(lb);
        (lighter + 0.05) / (darker + 0.05)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn rgb_to_xyz(rgb: Rgb) -> (f64, f64, f64) {
        fn linearise(c: u8) -> f64 {
            let v = c as f64 / 255.0;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        let r = linearise(rgb.r);
        let g = linearise(rgb.g);
        let b = linearise(rgb.b);

        // sRGB → XYZ (D65) matrix.
        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
        let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;
        (x, y, z)
    }

    fn xyz_to_rgb(x: f64, y: f64, z: f64) -> Rgb {
        // XYZ (D65) → sRGB matrix.
        let r_lin = x * 3.2404542 - y * 1.5371385 - z * 0.4985314;
        let g_lin = -x * 0.9692660 + y * 1.8760108 + z * 0.0415560;
        let b_lin = x * 0.0556434 - y * 0.2040259 + z * 1.0572252;

        fn gamma(v: f64) -> u8 {
            let v = v.clamp(0.0, 1.0);
            let encoded = if v <= 0.0031308 {
                12.92 * v
            } else {
                1.055 * v.powf(1.0 / 2.4) - 0.055
            };
            (encoded * 255.0).round() as u8
        }
        Rgb::new(gamma(r_lin), gamma(g_lin), gamma(b_lin))
    }

    fn f_lab(t: f64) -> f64 {
        let delta: f64 = 6.0 / 29.0;
        if t > delta.powi(3) {
            t.cbrt()
        } else {
            t / (3.0 * delta * delta) + 4.0 / 29.0
        }
    }

    fn f_lab_inv(t: f64) -> f64 {
        let delta: f64 = 6.0 / 29.0;
        if t > delta {
            t.powi(3)
        } else {
            3.0 * delta * delta * (t - 4.0 / 29.0)
        }
    }
}

// ---------------------------------------------------------------------------
// ColorHarmony
// ---------------------------------------------------------------------------

/// Rules for selecting related colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorHarmony {
    Complementary,
    Analogous,
    Triadic,
    Tetradic,
    SplitComplementary,
    Monochromatic,
}

// ---------------------------------------------------------------------------
// PaletteGenerator
// ---------------------------------------------------------------------------

pub struct PaletteGenerator;

impl PaletteGenerator {
    /// Generate `n` colors following the given harmony from `base`.
    pub fn generate(base: Rgb, harmony: ColorHarmony, n: usize) -> Vec<Rgb> {
        let hsv = ColorConversions::rgb_to_hsv(base);
        let hsvs: Vec<Hsv> = match harmony {
            ColorHarmony::Complementary => {
                let mut v = vec![hsv, Self::complementary(&hsv)];
                v.truncate(n);
                v
            }
            ColorHarmony::Analogous => {
                let spread = 30.0;
                let mut v = Self::analogous(&hsv, spread);
                while v.len() < n {
                    v.push(Hsv::new(hsv.h + v.len() as f64 * spread, hsv.s, hsv.v));
                }
                v.truncate(n);
                v
            }
            ColorHarmony::Triadic => {
                let mut v = Self::triadic(&hsv);
                v.truncate(n);
                v
            }
            ColorHarmony::Tetradic => {
                let mut v = vec![
                    hsv,
                    Hsv::new(hsv.h + 90.0, hsv.s, hsv.v),
                    Hsv::new(hsv.h + 180.0, hsv.s, hsv.v),
                    Hsv::new(hsv.h + 270.0, hsv.s, hsv.v),
                ];
                v.truncate(n);
                v
            }
            ColorHarmony::SplitComplementary => {
                let mut v = vec![
                    hsv,
                    Hsv::new(hsv.h + 150.0, hsv.s, hsv.v),
                    Hsv::new(hsv.h + 210.0, hsv.s, hsv.v),
                ];
                v.truncate(n);
                v
            }
            ColorHarmony::Monochromatic => Self::monochromatic(&hsv, n),
        };

        hsvs.into_iter().map(ColorConversions::hsv_to_rgb).collect()
    }

    /// Opposite hue (hue + 180°).
    pub fn complementary(base: &Hsv) -> Hsv {
        Hsv::new(base.h + 180.0, base.s, base.v)
    }

    /// ±`spread` degrees from the base hue.
    pub fn analogous(base: &Hsv, spread: f64) -> Vec<Hsv> {
        vec![
            Hsv::new(base.h - spread, base.s, base.v),
            *base,
            Hsv::new(base.h + spread, base.s, base.v),
        ]
    }

    /// Three hues equally spaced (hue ± 120°).
    pub fn triadic(base: &Hsv) -> Vec<Hsv> {
        vec![
            *base,
            Hsv::new(base.h + 120.0, base.s, base.v),
            Hsv::new(base.h + 240.0, base.s, base.v),
        ]
    }

    /// `n` colours with the same hue/saturation but varying value (lightness).
    pub fn monochromatic(base: &Hsv, n: usize) -> Vec<Hsv> {
        if n == 0 {
            return Vec::new();
        }
        (0..n)
            .map(|i| {
                let v = if n == 1 {
                    base.v
                } else {
                    0.2 + (i as f64 / (n - 1) as f64) * 0.8
                };
                Hsv::new(base.h, base.s, v)
            })
            .collect()
    }

    /// `n` colours equidistant in CIELAB hue at a given lightness.
    pub fn perceptually_uniform(n: usize, lightness: f64) -> Vec<Rgb> {
        if n == 0 {
            return Vec::new();
        }
        let chroma = 50.0; // reasonable chroma for sRGB gamut
        (0..n)
            .map(|i| {
                let angle = (i as f64 / n as f64) * 2.0 * std::f64::consts::PI;
                let lab = Lab {
                    l: lightness,
                    a: chroma * angle.cos(),
                    b: chroma * angle.sin(),
                };
                ColorConversions::lab_to_rgb(lab)
            })
            .collect()
    }

    /// `n` colours with a contrast ratio ≥ 4.5 against `background`.
    pub fn accessible_palette(background: &Rgb, n: usize) -> Vec<Rgb> {
        let candidates: Vec<Rgb> = (0..n * 20)
            .map(|i| {
                // Sample hues evenly with varying lightness.
                let hue = (i as f64 / (n as f64 * 20.0)) * 360.0;
                let lightness = if i % 2 == 0 { 0.15 } else { 0.85 };
                let hsv = Hsv::new(hue, 0.9, lightness);
                ColorConversions::hsv_to_rgb(hsv)
            })
            .filter(|c| ColorConversions::contrast_ratio(background, c) >= 4.5)
            .take(n)
            .collect();
        candidates
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_hsv_roundtrip() {
        let original = Rgb::new(123, 45, 200);
        let hsv = ColorConversions::rgb_to_hsv(original);
        let back = ColorConversions::hsv_to_rgb(hsv);
        // Roundtrip tolerance of ±2 due to rounding.
        assert!((back.r as i32 - original.r as i32).abs() <= 2);
        assert!((back.g as i32 - original.g as i32).abs() <= 2);
        assert!((back.b as i32 - original.b as i32).abs() <= 2);
    }

    #[test]
    fn rgb_hsl_roundtrip() {
        let original = Rgb::new(80, 160, 40);
        let hsl = ColorConversions::rgb_to_hsl(original);
        let back = ColorConversions::hsl_to_rgb(hsl);
        assert!((back.r as i32 - original.r as i32).abs() <= 2);
        assert!((back.g as i32 - original.g as i32).abs() <= 2);
        assert!((back.b as i32 - original.b as i32).abs() <= 2);
    }

    #[test]
    fn rgb_lab_roundtrip() {
        let original = Rgb::new(200, 100, 50);
        let lab = ColorConversions::rgb_to_lab(original);
        let back = ColorConversions::lab_to_rgb(lab);
        assert!((back.r as i32 - original.r as i32).abs() <= 3);
        assert!((back.g as i32 - original.g as i32).abs() <= 3);
        assert!((back.b as i32 - original.b as i32).abs() <= 3);
    }

    #[test]
    fn complementary_hue_diff_is_180() {
        let hsv = Hsv::new(60.0, 0.8, 0.9);
        let comp = PaletteGenerator::complementary(&hsv);
        let diff = (comp.h - hsv.h).rem_euclid(360.0);
        assert!((diff - 180.0).abs() < 1e-9);
    }

    #[test]
    fn triadic_has_three_colours() {
        let hsv = Hsv::new(0.0, 1.0, 1.0);
        let v = PaletteGenerator::triadic(&hsv);
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn monochromatic_varies_value() {
        let hsv = Hsv::new(120.0, 0.7, 0.5);
        let mono = PaletteGenerator::monochromatic(&hsv, 5);
        assert_eq!(mono.len(), 5);
        // All should share hue and saturation.
        for h in &mono {
            assert!((h.h - hsv.h).abs() < 1e-9);
            assert!((h.s - hsv.s).abs() < 1e-9);
        }
    }

    #[test]
    fn white_black_contrast_high() {
        let white = Rgb::new(255, 255, 255);
        let black = Rgb::new(0, 0, 0);
        let ratio = ColorConversions::contrast_ratio(&white, &black);
        assert!(ratio > 20.0);
    }

    #[test]
    fn luminance_white_is_one() {
        let white = Rgb::new(255, 255, 255);
        let lum = ColorConversions::luminance(&white);
        assert!((lum - 1.0).abs() < 1e-9);
    }

    #[test]
    fn luminance_black_is_zero() {
        let black = Rgb::new(0, 0, 0);
        let lum = ColorConversions::luminance(&black);
        assert!(lum.abs() < 1e-9);
    }

    #[test]
    fn color_distance_same_is_zero() {
        let lab = Lab {
            l: 50.0,
            a: 20.0,
            b: -10.0,
        };
        assert!(ColorConversions::color_distance(&lab, &lab) < 1e-9);
    }

    #[test]
    fn perceptually_uniform_count() {
        let colours = PaletteGenerator::perceptually_uniform(6, 60.0);
        assert_eq!(colours.len(), 6);
    }

    #[test]
    fn generate_complementary_palette() {
        let base = Rgb::new(255, 0, 0);
        let palette = PaletteGenerator::generate(base, ColorHarmony::Complementary, 2);
        assert_eq!(palette.len(), 2);
    }
}
