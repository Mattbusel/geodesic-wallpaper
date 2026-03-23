//! # Pattern Metamorphosis
//!
//! Morphing between two wallpaper pattern frames using pixel interpolation
//! and several easing functions.

use std::f64::consts::PI;

// ── EasingFn ──────────────────────────────────────────────────────────────────

/// Easing function family controlling the morph pacing over t ∈ [0, 1].
#[derive(Debug, Clone, PartialEq)]
pub enum EasingFn {
    /// Constant velocity: t.
    Linear,
    /// Smooth acceleration + deceleration.
    EaseInOut,
    /// Bouncy deceleration at the end.
    Bounce,
    /// Elastic overshoot.
    Elastic,
}

impl EasingFn {
    /// Map `t ∈ [0, 1]` through the easing curve, returning a value in [0, 1].
    pub fn apply(&self, t: f64) -> f64 {
        match self {
            EasingFn::Linear => t,
            EasingFn::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EasingFn::Bounce => {
                // Piecewise approximation of a bounce deceleration.
                if t < 1.0 / 2.75 {
                    7.5625 * t * t
                } else if t < 2.0 / 2.75 {
                    let t = t - 1.5 / 2.75;
                    7.5625 * t * t + 0.75
                } else if t < 2.5 / 2.75 {
                    let t = t - 2.25 / 2.75;
                    7.5625 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / 2.75;
                    7.5625 * t * t + 0.984375
                }
            }
            EasingFn::Elastic => {
                if t == 0.0 || t == 1.0 {
                    return t;
                }
                -(2.0_f64.powf(10.0 * t - 10.0)) * ((10.0 * t - 10.75) * (2.0 * PI / 3.0)).sin()
            }
        }
    }
}

// ── MorphConfig ───────────────────────────────────────────────────────────────

/// Configuration for a morph sequence.
#[derive(Debug, Clone)]
pub struct MorphConfig {
    /// Number of intermediate frames to generate (not counting endpoints).
    pub steps: usize,
    /// Easing function applied to the morph parameter.
    pub easing: EasingFn,
}

impl Default for MorphConfig {
    fn default() -> Self {
        Self {
            steps: 16,
            easing: EasingFn::EaseInOut,
        }
    }
}

// ── PixelMorph ────────────────────────────────────────────────────────────────

/// Pixel-level morphing utilities.
pub struct PixelMorph;

impl PixelMorph {
    /// Linear interpolation of two RGB colours.
    ///
    /// `t = 0` → `a`, `t = 1` → `b`.
    pub fn lerp_rgb(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
        (lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2))
    }

    /// Generate `steps` intermediate RGB frames by lerping each pixel from
    /// `frame_a` to `frame_b`.
    ///
    /// The returned vector has exactly `steps` frames; parameter t for frame i
    /// is `(i + 1) / (steps + 1)` so the endpoints are not included.
    pub fn morph_frames(
        frame_a: &[u8],
        frame_b: &[u8],
        width: usize,
        height: usize,
        steps: usize,
        easing: &EasingFn,
    ) -> Vec<Vec<u8>> {
        let pixel_count = width * height;
        assert_eq!(frame_a.len(), pixel_count * 3, "frame_a size mismatch");
        assert_eq!(frame_b.len(), pixel_count * 3, "frame_b size mismatch");

        (1..=steps)
            .map(|i| {
                let raw_t = i as f64 / (steps + 1) as f64;
                let t = easing.apply(raw_t);
                frame_a
                    .chunks(3)
                    .zip(frame_b.chunks(3))
                    .flat_map(|(pa, pb)| {
                        let ca = (pa[0], pa[1], pa[2]);
                        let cb = (pb[0], pb[1], pb[2]);
                        let (r, g, b) = Self::lerp_rgb(ca, cb, t);
                        [r, g, b]
                    })
                    .collect()
            })
            .collect()
    }
}

// ── PatternMorpher ────────────────────────────────────────────────────────────

/// Higher-level morph generator that combines crossfade and warp blending.
pub struct PatternMorpher;

impl PatternMorpher {
    /// Single crossfade frame at parameter `t ∈ [0, 1]`.
    pub fn crossfade(a: &[u8], b: &[u8], t: f64, _w: usize, _h: usize) -> Vec<u8> {
        a.chunks(3)
            .zip(b.chunks(3))
            .flat_map(|(pa, pb)| {
                let ca = (pa[0], pa[1], pa[2]);
                let cb = (pb[0], pb[1], pb[2]);
                let (r, g, b_ch) = PixelMorph::lerp_rgb(ca, cb, t);
                [r, g, b_ch]
            })
            .collect()
    }

    /// Warp-morph frame: pixels of `b` are horizontally displaced by
    /// `t * sin(x / w * π) * 10` pixels before blending with `a`.
    pub fn warp_morph(a: &[u8], b: &[u8], t: f64, w: usize, h: usize) -> Vec<u8> {
        let t_c = t.clamp(0.0, 1.0);
        let mut out = vec![0u8; w * h * 3];

        for y in 0..h {
            for x in 0..w {
                // Compute displaced x in frame b.
                let disp = t_c * ((x as f64 / w as f64) * PI).sin() * 10.0;
                let src_x = (x as f64 + disp).round() as i64;
                let src_x = src_x.clamp(0, w as i64 - 1) as usize;

                let dst_idx = (y * w + x) * 3;
                let src_b_idx = (y * w + src_x) * 3;

                let ca = (a[dst_idx], a[dst_idx + 1], a[dst_idx + 2]);
                let cb = if src_b_idx + 2 < b.len() {
                    (b[src_b_idx], b[src_b_idx + 1], b[src_b_idx + 2])
                } else {
                    (0, 0, 0)
                };

                let (r, g, b_ch) = PixelMorph::lerp_rgb(ca, cb, t_c);
                out[dst_idx]     = r;
                out[dst_idx + 1] = g;
                out[dst_idx + 2] = b_ch;
            }
        }
        out
    }

    /// Full morph sequence from `a` to `b` using `config.steps` intermediate
    /// frames.
    pub fn generate_sequence(a: &[u8], b: &[u8], config: &MorphConfig) -> Vec<Vec<u8>> {
        PixelMorph::morph_frames(a, b, 1, a.len() / 3, config.steps, &config.easing)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_frame(w: usize, h: usize, r: u8, g: u8, b: u8) -> Vec<u8> {
        vec![[r, g, b]; w * h].into_iter().flatten().collect()
    }

    #[test]
    fn lerp_at_t0_returns_a() {
        let a = (100u8, 150u8, 200u8);
        let b = (0u8, 50u8, 100u8);
        assert_eq!(PixelMorph::lerp_rgb(a, b, 0.0), a);
    }

    #[test]
    fn lerp_at_t1_returns_b() {
        let a = (100u8, 150u8, 200u8);
        let b = (0u8, 50u8, 100u8);
        assert_eq!(PixelMorph::lerp_rgb(a, b, 1.0), b);
    }

    #[test]
    fn lerp_midpoint() {
        let a = (0u8, 0u8, 0u8);
        let b = (100u8, 200u8, 100u8);
        let mid = PixelMorph::lerp_rgb(a, b, 0.5);
        assert_eq!(mid, (50, 100, 50));
    }

    #[test]
    fn morph_frames_correct_step_count() {
        let w = 4;
        let h = 4;
        let a = solid_frame(w, h, 0, 0, 0);
        let b = solid_frame(w, h, 255, 255, 255);
        let frames = PixelMorph::morph_frames(&a, &b, w, h, 8, &EasingFn::Linear);
        assert_eq!(frames.len(), 8);
    }

    #[test]
    fn easing_linear_in_range() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            let v = EasingFn::Linear.apply(t);
            assert!((v - t).abs() < 1e-10);
        }
    }

    #[test]
    fn easing_functions_in_unit_range() {
        let fns = [
            EasingFn::Linear,
            EasingFn::EaseInOut,
            EasingFn::Bounce,
        ];
        for ease in &fns {
            for i in 0..=20 {
                let t = i as f64 / 20.0;
                let v = ease.apply(t);
                assert!(
                    v >= -0.01 && v <= 1.01,
                    "{ease:?} out of range at t={t}: {v}"
                );
            }
        }
    }

    #[test]
    fn elastic_at_endpoints() {
        assert_eq!(EasingFn::Elastic.apply(0.0), 0.0);
        assert_eq!(EasingFn::Elastic.apply(1.0), 1.0);
    }

    #[test]
    fn crossfade_at_endpoints() {
        let a = solid_frame(2, 2, 255, 0, 0);
        let b = solid_frame(2, 2, 0, 0, 255);
        let fa = PatternMorpher::crossfade(&a, &b, 0.0, 2, 2);
        let fb = PatternMorpher::crossfade(&a, &b, 1.0, 2, 2);
        assert_eq!(fa, a);
        assert_eq!(fb, b);
    }

    #[test]
    fn warp_morph_correct_size() {
        let w = 8;
        let h = 8;
        let a = solid_frame(w, h, 100, 100, 100);
        let b = solid_frame(w, h, 200, 200, 200);
        let frame = PatternMorpher::warp_morph(&a, &b, 0.5, w, h);
        assert_eq!(frame.len(), w * h * 3);
    }

    #[test]
    fn generate_sequence_step_count() {
        let a = solid_frame(4, 1, 0, 0, 0);
        let b = solid_frame(4, 1, 255, 255, 255);
        let config = MorphConfig {
            steps: 5,
            easing: EasingFn::Linear,
        };
        let seq = PatternMorpher::generate_sequence(&a, &b, &config);
        assert_eq!(seq.len(), 5);
    }
}
