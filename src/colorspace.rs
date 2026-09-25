//! Color Space Transforms
//!
//! Full color space conversion library: RGB, HSV, Lab (CIE 1976), Oklab.
//! Includes smooth interpolation in each color space.

// ── Color types ───────────────────────────────────────────────────────────────

/// 8-bit linear RGB.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// HSV (Hue, Saturation, Value).
/// h ∈ [0, 360), s ∈ [0, 1], v ∈ [0, 1].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// CIE Lab (D65 illuminant).
/// l ∈ [0, 100], a ∈ [-128, 127], b ∈ [-128, 127].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

/// Björn Ottosson's Oklab perceptual color space.
/// l ∈ [0, 1], a ∈ [-0.5, 0.5], b ∈ [-0.5, 0.5].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oklab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

// ── RGB ↔ HSV ─────────────────────────────────────────────────────────────────

/// Convert sRGB to HSV.
pub fn rgb_to_hsv(rgb: Rgb) -> Hsv {
    let r = rgb.r as f32 / 255.0;
    let g = rgb.g as f32 / 255.0;
    let b = rgb.b as f32 / 255.0;

    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;

    let h = if delta.abs() < f32::EPSILON {
        0.0
    } else if cmax == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if cmax == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };

    let s = if cmax < f32::EPSILON {
        0.0
    } else {
        delta / cmax
    };
    let v = cmax;

    Hsv { h, s, v }
}

/// Convert HSV to sRGB.
pub fn hsv_to_rgb(hsv: Hsv) -> Rgb {
    let h = ((hsv.h % 360.0) + 360.0) % 360.0;
    let s = hsv.s.clamp(0.0, 1.0);
    let v = hsv.v.clamp(0.0, 1.0);

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

    Rgb {
        r: ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        g: ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        b: ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}

// ── RGB ↔ Lab ─────────────────────────────────────────────────────────────────

/// Convert sRGB to CIE Lab (D65 illuminant).
pub fn rgb_to_lab(rgb: Rgb) -> Lab {
    let xyz = rgb_to_xyz(rgb);
    xyz_to_lab(xyz)
}

/// Convert CIE Lab to sRGB (D65 illuminant).
pub fn lab_to_rgb(lab: Lab) -> Rgb {
    let xyz = lab_to_xyz(lab);
    xyz_to_rgb(xyz)
}

// ── RGB ↔ Oklab ───────────────────────────────────────────────────────────────

/// Convert sRGB to Oklab (Björn Ottosson's perceptual color space).
///
/// Reference: <https://bottosson.github.io/posts/oklab/>
pub fn rgb_to_oklab(rgb: Rgb) -> Oklab {
    let r = srgb_to_linear(rgb.r as f32 / 255.0);
    let g = srgb_to_linear(rgb.g as f32 / 255.0);
    let b = srgb_to_linear(rgb.b as f32 / 255.0);

    // Step 1: linear sRGB → LMS (M1 matrix from Ottosson)
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    // Step 2: cube root
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // Step 3: LMS_ → Lab (M2 matrix)
    let ok_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let ok_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let ok_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    Oklab {
        l: ok_l,
        a: ok_a,
        b: ok_b,
    }
}

/// Convert Oklab to sRGB.
pub fn oklab_to_rgb(oklab: Oklab) -> Rgb {
    // Inverse M2
    let l_ = oklab.l + 0.3963377774 * oklab.a + 0.2158037573 * oklab.b;
    let m_ = oklab.l - 0.1055613458 * oklab.a - 0.0638541728 * oklab.b;
    let s_ = oklab.l - 0.0894841775 * oklab.a - 1.2914855480 * oklab.b;

    // Cube
    let l = l_ * l_ * l_;
    let m = m_ * m_ * m_;
    let s = s_ * s_ * s_;

    // Inverse M1
    let r = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
    let g = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
    let b = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

    Rgb {
        r: (linear_to_srgb(r) * 255.0).round().clamp(0.0, 255.0) as u8,
        g: (linear_to_srgb(g) * 255.0).round().clamp(0.0, 255.0) as u8,
        b: (linear_to_srgb(b) * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}

// ── ColorInterpolator ─────────────────────────────────────────────────────────

/// Smooth color interpolation in various color spaces.
pub struct ColorInterpolator;

impl ColorInterpolator {
    /// Linear interpolation in sRGB space.
    pub fn lerp_rgb(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        Rgb {
            r: lerp_u8(a.r, b.r, t),
            g: lerp_u8(a.g, b.g, t),
            b: lerp_u8(a.b, b.b, t),
        }
    }

    /// Interpolation in HSV space with hue-aware shortest-arc blending.
    pub fn lerp_hsv(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let ha = rgb_to_hsv(a);
        let hb = rgb_to_hsv(b);

        // Shortest arc for hue
        let mut dh = hb.h - ha.h;
        if dh > 180.0 {
            dh -= 360.0;
        }
        if dh < -180.0 {
            dh += 360.0;
        }
        let h = (ha.h + dh * t + 360.0) % 360.0;
        let s = ha.s + (hb.s - ha.s) * t;
        let v = ha.v + (hb.v - ha.v) * t;

        hsv_to_rgb(Hsv { h, s, v })
    }

    /// Perceptually uniform interpolation in Oklab space.
    pub fn lerp_oklab(a: Rgb, b: Rgb, t: f32) -> Rgb {
        let t = t.clamp(0.0, 1.0);
        let oa = rgb_to_oklab(a);
        let ob = rgb_to_oklab(b);
        let interp = Oklab {
            l: oa.l + (ob.l - oa.l) * t,
            a: oa.a + (ob.a - oa.a) * t,
            b: oa.b + (ob.b - oa.b) * t,
        };
        oklab_to_rgb(interp)
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[inline]
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// sRGB gamma → linear (IEC 61966-2-1).
#[inline]
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear → sRGB gamma.
#[inline]
fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert sRGB to CIE XYZ (D65).
fn rgb_to_xyz(rgb: Rgb) -> (f32, f32, f32) {
    let r = srgb_to_linear(rgb.r as f32 / 255.0);
    let g = srgb_to_linear(rgb.g as f32 / 255.0);
    let b = srgb_to_linear(rgb.b as f32 / 255.0);

    let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;
    (x, y, z)
}

/// Convert CIE XYZ (D65) to sRGB.
fn xyz_to_rgb(xyz: (f32, f32, f32)) -> Rgb {
    let (x, y, z) = xyz;
    let r = x * 3.2404542 + y * -1.5371385 + z * -0.4985314;
    let g = x * -0.9692660 + y * 1.8760108 + z * 0.0415560;
    let b = x * 0.0556434 + y * -0.2040259 + z * 1.0572252;

    Rgb {
        r: (linear_to_srgb(r) * 255.0).round().clamp(0.0, 255.0) as u8,
        g: (linear_to_srgb(g) * 255.0).round().clamp(0.0, 255.0) as u8,
        b: (linear_to_srgb(b) * 255.0).round().clamp(0.0, 255.0) as u8,
    }
}

/// CIE XYZ to Lab using D65 white point.
fn xyz_to_lab(xyz: (f32, f32, f32)) -> Lab {
    // D65 reference white
    let xn = 0.950489;
    let yn = 1.000000;
    let zn = 1.088840;

    let fx = lab_f(xyz.0 / xn);
    let fy = lab_f(xyz.1 / yn);
    let fz = lab_f(xyz.2 / zn);

    Lab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

fn lab_f(t: f32) -> f32 {
    let delta: f32 = 6.0 / 29.0;
    if t > delta * delta * delta {
        t.cbrt()
    } else {
        t / (3.0 * delta * delta) + 4.0 / 29.0
    }
}

fn lab_f_inv(t: f32) -> f32 {
    let delta: f32 = 6.0 / 29.0;
    if t > delta {
        t * t * t
    } else {
        3.0 * delta * delta * (t - 4.0 / 29.0)
    }
}

fn lab_to_xyz(lab: Lab) -> (f32, f32, f32) {
    let xn = 0.950489;
    let yn = 1.000000;
    let zn = 1.088840;

    let fy = (lab.l + 16.0) / 116.0;
    let fx = fy + lab.a / 500.0;
    let fz = fy - lab.b / 200.0;

    (xn * lab_f_inv(fx), yn * lab_f_inv(fy), zn * lab_f_inv(fz))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq_rgb(a: Rgb, b: Rgb, tol: u8) -> bool {
        a.r.abs_diff(b.r) <= tol && a.g.abs_diff(b.g) <= tol && a.b.abs_diff(b.b) <= tol
    }

    // 1. RGB -> HSV -> RGB round-trip (black)
    #[test]
    fn test_rgb_hsv_round_trip_black() {
        let rgb = Rgb { r: 0, g: 0, b: 0 };
        let hsv = rgb_to_hsv(rgb);
        let out = hsv_to_rgb(hsv);
        assert!(approx_eq_rgb(rgb, out, 1));
    }

    // 2. RGB -> HSV -> RGB round-trip (white)
    #[test]
    fn test_rgb_hsv_round_trip_white() {
        let rgb = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let out = hsv_to_rgb(rgb_to_hsv(rgb));
        assert!(approx_eq_rgb(rgb, out, 1));
    }

    // 3. RGB -> HSV -> RGB round-trip (red)
    #[test]
    fn test_rgb_hsv_round_trip_red() {
        let rgb = Rgb { r: 255, g: 0, b: 0 };
        let out = hsv_to_rgb(rgb_to_hsv(rgb));
        assert!(approx_eq_rgb(rgb, out, 1));
    }

    // 4. Pure red has hue ~0
    #[test]
    fn test_red_hue_zero() {
        let hsv = rgb_to_hsv(Rgb { r: 255, g: 0, b: 0 });
        assert!(hsv.h < 1.0 || hsv.h > 359.0);
        assert!((hsv.s - 1.0).abs() < 0.01);
        assert!((hsv.v - 1.0).abs() < 0.01);
    }

    // 5. Pure green has hue ~120
    #[test]
    fn test_green_hue_120() {
        let hsv = rgb_to_hsv(Rgb { r: 0, g: 255, b: 0 });
        assert!((hsv.h - 120.0).abs() < 1.0);
    }

    // 6. RGB -> Lab -> RGB round-trip (white)
    #[test]
    fn test_rgb_lab_round_trip_white() {
        let rgb = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let out = lab_to_rgb(rgb_to_lab(rgb));
        assert!(approx_eq_rgb(rgb, out, 2));
    }

    // 7. RGB -> Lab -> RGB round-trip (mid-grey)
    #[test]
    fn test_rgb_lab_round_trip_grey() {
        let rgb = Rgb {
            r: 128,
            g: 128,
            b: 128,
        };
        let out = lab_to_rgb(rgb_to_lab(rgb));
        assert!(approx_eq_rgb(rgb, out, 2));
    }

    // 8. RGB -> Oklab -> RGB round-trip (white)
    #[test]
    fn test_rgb_oklab_round_trip_white() {
        let rgb = Rgb {
            r: 255,
            g: 255,
            b: 255,
        };
        let out = oklab_to_rgb(rgb_to_oklab(rgb));
        assert!(approx_eq_rgb(rgb, out, 2));
    }

    // 9. RGB -> Oklab -> RGB round-trip (black)
    #[test]
    fn test_rgb_oklab_round_trip_black() {
        let rgb = Rgb { r: 0, g: 0, b: 0 };
        let out = oklab_to_rgb(rgb_to_oklab(rgb));
        assert!(approx_eq_rgb(rgb, out, 2));
    }

    // 10. Oklab L for white is near 1.0
    #[test]
    fn test_oklab_white_l() {
        let ok = rgb_to_oklab(Rgb {
            r: 255,
            g: 255,
            b: 255,
        });
        assert!((ok.l - 1.0).abs() < 0.01);
    }

    // 11. Oklab L for black is near 0.0
    #[test]
    fn test_oklab_black_l() {
        let ok = rgb_to_oklab(Rgb { r: 0, g: 0, b: 0 });
        assert!(ok.l.abs() < 0.01);
    }

    // 12. lerp_rgb at t=0 returns a
    #[test]
    fn test_lerp_rgb_zero() {
        let a = Rgb {
            r: 10,
            g: 20,
            b: 30,
        };
        let b = Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        assert_eq!(ColorInterpolator::lerp_rgb(a, b, 0.0), a);
    }

    // 13. lerp_rgb at t=1 returns b
    #[test]
    fn test_lerp_rgb_one() {
        let a = Rgb {
            r: 10,
            g: 20,
            b: 30,
        };
        let b = Rgb {
            r: 200,
            g: 100,
            b: 50,
        };
        assert_eq!(ColorInterpolator::lerp_rgb(a, b, 1.0), b);
    }

    // 14. lerp_rgb midpoint
    #[test]
    fn test_lerp_rgb_midpoint() {
        let a = Rgb { r: 0, g: 0, b: 0 };
        let b = Rgb {
            r: 100,
            g: 100,
            b: 100,
        };
        let mid = ColorInterpolator::lerp_rgb(a, b, 0.5);
        assert!((mid.r as i16 - 50).abs() <= 1);
    }

    // 15. lerp_hsv hue wraps correctly (red → blue shortest arc)
    #[test]
    fn test_lerp_hsv_hue_wrap() {
        // Red (h=0) and Magenta (h=300) → shortest arc goes through 330 (pink)
        let red = Rgb { r: 255, g: 0, b: 0 };
        let magenta = Rgb {
            r: 255,
            g: 0,
            b: 255,
        };
        let mid = ColorInterpolator::lerp_hsv(red, magenta, 0.5);
        // At t=0.5 we expect a pinkish/magenta colour, not green
        assert!(mid.b > 100, "blue component should be high");
    }

    // 16. lerp_oklab returns valid RGB
    #[test]
    fn test_lerp_oklab_valid_rgb() {
        let a = Rgb {
            r: 100,
            g: 50,
            b: 200,
        };
        let b = Rgb {
            r: 200,
            g: 150,
            b: 50,
        };
        let mid = ColorInterpolator::lerp_oklab(a, b, 0.5);
        // Just check it doesn't panic and returns plausible values
        assert!(mid.r <= 255 && mid.g <= 255 && mid.b <= 255);
    }

    // 17. Lab L for white is ~100
    #[test]
    fn test_lab_white_l() {
        let lab = rgb_to_lab(Rgb {
            r: 255,
            g: 255,
            b: 255,
        });
        assert!((lab.l - 100.0).abs() < 1.0);
    }

    // 18. Lab L for black is ~0
    #[test]
    fn test_lab_black_l() {
        let lab = rgb_to_lab(Rgb { r: 0, g: 0, b: 0 });
        assert!(lab.l.abs() < 1.0);
    }

    // 19. HSV saturation of white is 0
    #[test]
    fn test_white_saturation_zero() {
        let hsv = rgb_to_hsv(Rgb {
            r: 255,
            g: 255,
            b: 255,
        });
        assert!(hsv.s < 0.01);
    }

    // 20. srgb_to_linear(1.0) = 1.0
    #[test]
    fn test_srgb_linear_one() {
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 0.001);
    }

    // 21. RGB -> Oklab -> RGB round-trip (colour)
    #[test]
    fn test_rgb_oklab_round_trip_colour() {
        let rgb = Rgb {
            r: 180,
            g: 100,
            b: 60,
        };
        let out = oklab_to_rgb(rgb_to_oklab(rgb));
        assert!(approx_eq_rgb(rgb, out, 3));
    }

    // 22. RGB -> Lab -> RGB round-trip (colour)
    #[test]
    fn test_rgb_lab_round_trip_colour() {
        let rgb = Rgb {
            r: 120,
            g: 60,
            b: 200,
        };
        let out = lab_to_rgb(rgb_to_lab(rgb));
        assert!(approx_eq_rgb(rgb, out, 3));
    }
}
