//! Frame-by-frame animation engine with keyframe tweening and easing functions.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// EasingFunction
// ---------------------------------------------------------------------------

/// A family of easing curves that remap a normalised time parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum EasingFunction {
    /// No easing: progress equals time.
    Linear,
    /// Quadratic ease-in: starts slow, speeds up.
    EaseIn,
    /// Quadratic ease-out: starts fast, slows down.
    EaseOut,
    /// Quadratic ease-in for the first half, ease-out for the second.
    EaseInOut,
    /// Ease-out bounce: settles with decaying bounces at the end.
    Bounce,
    /// Elastic ease-in: winds up with a growing oscillation before the end.
    Elastic,
    /// Back ease-in: dips slightly below 0 before moving forward.
    Back,
    /// Cubic ease-in-out.
    Cubic,
}

impl EasingFunction {
    /// Map `t` ∈ [0, 1] through the easing curve, returning a value in [0, 1]
    /// (some overshooting curves may briefly exceed this range).
    pub fn apply(t: f64, easing: &EasingFunction) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match easing {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn => t * t,
            EasingFunction::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EasingFunction::Cubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            EasingFunction::Bounce => {
                // Ease-out bounce
                let d1 = 2.75_f64;
                let n1 = 7.5625_f64;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t2 = t - 1.5 / d1;
                    n1 * t2 * t2 + 0.75
                } else if t < 2.5 / d1 {
                    let t2 = t - 2.25 / d1;
                    n1 * t2 * t2 + 0.9375
                } else {
                    let t2 = t - 2.625 / d1;
                    n1 * t2 * t2 + 0.984375
                }
            }
            EasingFunction::Elastic => {
                if t == 0.0 || t == 1.0 {
                    return t;
                }
                let c4 = 2.0 * std::f64::consts::PI / 3.0;
                -(2.0_f64.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * c4).sin()
            }
            EasingFunction::Back => {
                let c1 = 1.70158_f64;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            }
        }
    }
}

// ---------------------------------------------------------------------------
// TweenTarget
// ---------------------------------------------------------------------------

/// A value that can be interpolated between keyframes.
#[derive(Debug, Clone, PartialEq)]
pub enum TweenTarget {
    /// A single scalar value.
    Float(f64),
    /// An RGB colour, interpolated per channel.
    Color([u8; 3]),
    /// A 2D point `(x, y)`.
    Vec2(f64, f64),
}

// ---------------------------------------------------------------------------
// Keyframe
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
/// A value pinned to a point in time on an [`AnimTrack`].
pub struct Keyframe {
    /// Time of this keyframe in seconds from the start of the clip.
    pub time_secs: f64,
    /// Value the property takes at `time_secs`.
    pub value: TweenTarget,
    /// Easing applied on the segment that ends at this keyframe.
    pub easing: EasingFunction,
}

// ---------------------------------------------------------------------------
// AnimTrack
// ---------------------------------------------------------------------------

/// A single animated property over time, defined by sorted keyframes.
#[derive(Debug, Clone)]
pub struct AnimTrack {
    /// Name of the animated property (the key in the state map).
    pub property: String,
    /// Keyframes, kept sorted by `time_secs`.
    pub keyframes: Vec<Keyframe>,
}

impl AnimTrack {
    /// Create an empty track for the property named `property`.
    pub fn new(property: impl Into<String>) -> Self {
        Self {
            property: property.into(),
            keyframes: Vec::new(),
        }
    }

    /// Insert a keyframe, keeping the track sorted by time.
    pub fn add_keyframe(&mut self, kf: Keyframe) {
        self.keyframes.push(kf);
        self.keyframes
            .sort_by(|a, b| a.time_secs.total_cmp(&b.time_secs));
    }

    /// Interpolate the track value at `time_secs`.
    /// Returns `None` if the track has no keyframes or time is out of range.
    pub fn value_at(&self, time_secs: f64) -> Option<TweenTarget> {
        if self.keyframes.is_empty() {
            return None;
        }
        // Before first keyframe → hold first value.
        if time_secs <= self.keyframes[0].time_secs {
            return Some(self.keyframes[0].value.clone());
        }
        // After last keyframe → hold last value.
        let last = self.keyframes.last().unwrap();
        if time_secs >= last.time_secs {
            return Some(last.value.clone());
        }
        // Find surrounding keyframes.
        for i in 0..self.keyframes.len() - 1 {
            let a = &self.keyframes[i];
            let b = &self.keyframes[i + 1];
            if time_secs >= a.time_secs && time_secs <= b.time_secs {
                let span = b.time_secs - a.time_secs;
                let raw_t = if span < 1e-12 {
                    1.0
                } else {
                    (time_secs - a.time_secs) / span
                };
                let t = EasingFunction::apply(raw_t, &b.easing);
                return Some(Self::lerp_tween(&a.value, &b.value, t));
            }
        }
        None
    }

    /// Linear interpolation between two `TweenTarget` values.
    pub fn lerp_tween(a: &TweenTarget, b: &TweenTarget, t: f64) -> TweenTarget {
        match (a, b) {
            (TweenTarget::Float(fa), TweenTarget::Float(fb)) => {
                TweenTarget::Float(fa + (fb - fa) * t)
            }
            (TweenTarget::Color(ca), TweenTarget::Color(cb)) => {
                let r = (ca[0] as f64 + (cb[0] as f64 - ca[0] as f64) * t).round() as u8;
                let g = (ca[1] as f64 + (cb[1] as f64 - ca[1] as f64) * t).round() as u8;
                let b = (ca[2] as f64 + (cb[2] as f64 - ca[2] as f64) * t).round() as u8;
                TweenTarget::Color([r, g, b])
            }
            (TweenTarget::Vec2(ax, ay), TweenTarget::Vec2(bx, by)) => {
                TweenTarget::Vec2(ax + (bx - ax) * t, ay + (by - ay) * t)
            }
            // Mismatched types: snap to b at t ≥ 0.5.
            _ => {
                if t >= 0.5 {
                    b.clone()
                } else {
                    a.clone()
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AnimClip
// ---------------------------------------------------------------------------

/// A named animation clip containing multiple tracks.
#[derive(Debug, Clone)]
pub struct AnimClip {
    /// Clip name, used to look it up in [`AnimationEngine`].
    pub name: String,
    /// Length of the clip in seconds.
    pub duration_secs: f64,
    /// Animated properties in this clip.
    pub tracks: Vec<AnimTrack>,
    /// Whether time wraps around at `duration_secs` instead of holding the last frame.
    pub loop_: bool,
}

impl AnimClip {
    /// Create an empty clip with the given name, length in seconds and loop flag.
    pub fn new(name: impl Into<String>, duration_secs: f64, loop_: bool) -> Self {
        Self {
            name: name.into(),
            duration_secs,
            tracks: Vec::new(),
            loop_,
        }
    }

    /// Add a track to the clip.
    pub fn add_track(&mut self, track: AnimTrack) {
        self.tracks.push(track);
    }

    /// Evaluate all tracks at `time_secs`, returning a property→value map.
    pub fn state_at(&self, time_secs: f64) -> HashMap<String, TweenTarget> {
        let t = if self.loop_ && self.duration_secs > 0.0 {
            time_secs % self.duration_secs
        } else {
            time_secs.min(self.duration_secs)
        };
        let mut state = HashMap::new();
        for track in &self.tracks {
            if let Some(value) = track.value_at(t) {
                state.insert(track.property.clone(), value);
            }
        }
        state
    }
}

// ---------------------------------------------------------------------------
// FrameRenderer trait
// ---------------------------------------------------------------------------

/// Draws one frame of an animation from the evaluated property values.
pub trait FrameRenderer {
    /// Render a single frame given the animated state, returning an RGB pixel grid.
    fn render_frame(
        &self,
        state: &HashMap<String, TweenTarget>,
        width: u32,
        height: u32,
    ) -> Vec<Vec<[u8; 3]>>;
}

// ---------------------------------------------------------------------------
// AnimMetadata
// ---------------------------------------------------------------------------

/// Summary information about a rendered animation clip.
#[derive(Debug, Clone)]
pub struct AnimMetadata {
    /// Number of frames rendered.
    pub frame_count: usize,
    /// Frames per second the clip was rendered at.
    pub fps: f64,
    /// Clip length in seconds.
    pub duration_secs: f64,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
}

// ---------------------------------------------------------------------------
// AnimationEngine
// ---------------------------------------------------------------------------

/// Manages a collection of animation clips and drives rendering.
pub struct AnimationEngine {
    /// Clips available for rendering.
    pub clips: Vec<AnimClip>,
    /// Frames per second used when rendering clips.
    pub fps: f64,
    /// Output frame width in pixels.
    pub width: u32,
    /// Output frame height in pixels.
    pub height: u32,
}

impl AnimationEngine {
    /// Create an engine with no clips that renders at `fps` and `width` x `height` pixels.
    pub fn new(fps: f64, width: u32, height: u32) -> Self {
        Self {
            clips: Vec::new(),
            fps,
            width,
            height,
        }
    }

    /// Add a clip that can then be rendered by name.
    pub fn add_clip(&mut self, clip: AnimClip) {
        self.clips.push(clip);
    }

    fn find_clip(&self, name: &str) -> Option<&AnimClip> {
        self.clips.iter().find(|c| c.name == name)
    }

    /// Render every frame of a named clip using the provided renderer.
    pub fn render_clip(
        &self,
        clip_name: &str,
        renderer: &dyn FrameRenderer,
    ) -> Vec<Vec<Vec<[u8; 3]>>> {
        let Some(clip) = self.find_clip(clip_name) else {
            return Vec::new();
        };
        let frame_count = (clip.duration_secs * self.fps).ceil() as usize;
        (0..frame_count)
            .map(|i| {
                let t = i as f64 / self.fps;
                let state = clip.state_at(t);
                renderer.render_frame(&state, self.width, self.height)
            })
            .collect()
    }

    /// Render a single frame of a named clip at a specific time.
    pub fn render_frame_at(
        &self,
        clip_name: &str,
        time_secs: f64,
        renderer: &dyn FrameRenderer,
    ) -> Option<Vec<Vec<[u8; 3]>>> {
        let clip = self.find_clip(clip_name)?;
        let state = clip.state_at(time_secs);
        Some(renderer.render_frame(&state, self.width, self.height))
    }

    /// Convert an RGB frame into an ASCII art string using a brightness ramp.
    pub fn frame_to_ascii(frame: &[Vec<[u8; 3]>]) -> String {
        const RAMP: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        let mut out = String::new();
        for row in frame {
            for pixel in row {
                let lum =
                    0.299 * pixel[0] as f64 + 0.587 * pixel[1] as f64 + 0.114 * pixel[2] as f64;
                let idx = ((lum / 255.0) * (RAMP.len() - 1) as f64).round() as usize;
                out.push(RAMP[idx.min(RAMP.len() - 1)]);
            }
            out.push('\n');
        }
        out
    }

    /// Return metadata for a named clip.
    pub fn frames_to_gif_metadata(&self, clip_name: &str) -> AnimMetadata {
        let clip = self.find_clip(clip_name);
        let duration_secs = clip.map(|c| c.duration_secs).unwrap_or(0.0);
        let frame_count = (duration_secs * self.fps).ceil() as usize;
        AnimMetadata {
            frame_count,
            fps: self.fps,
            duration_secs,
            width: self.width,
            height: self.height,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    // A NaN keyframe time used to panic inside the sort (partial_cmp unwrap).
    #[test]
    fn nan_keyframe_time_does_not_panic() {
        let mut track = float_track("x", 0.0, 1.0, 2.0);
        track.add_keyframe(Keyframe {
            time_secs: f64::NAN,
            value: TweenTarget::Float(5.0),
            easing: EasingFunction::Linear,
        });
        assert_eq!(track.keyframes.len(), 3);
        let _ = track.value_at(1.0);
    }

    use super::*;

    fn float_track(property: &str, from: f64, to: f64, duration: f64) -> AnimTrack {
        let mut track = AnimTrack::new(property);
        track.add_keyframe(Keyframe {
            time_secs: 0.0,
            value: TweenTarget::Float(from),
            easing: EasingFunction::Linear,
        });
        track.add_keyframe(Keyframe {
            time_secs: duration,
            value: TweenTarget::Float(to),
            easing: EasingFunction::Linear,
        });
        track
    }

    #[test]
    fn linear_easing_midpoint() {
        let v = EasingFunction::apply(0.5, &EasingFunction::Linear);
        assert!((v - 0.5).abs() < 1e-9);
    }

    #[test]
    fn ease_in_less_than_half_at_midpoint() {
        let v = EasingFunction::apply(0.5, &EasingFunction::EaseIn);
        assert!(v < 0.5);
    }

    #[test]
    fn ease_out_more_than_half_at_midpoint() {
        let v = EasingFunction::apply(0.5, &EasingFunction::EaseOut);
        assert!(v > 0.5);
    }

    #[test]
    fn track_value_at_start_and_end() {
        let track = float_track("x", 0.0, 10.0, 1.0);
        let start = track.value_at(0.0).unwrap();
        let end = track.value_at(1.0).unwrap();
        assert_eq!(start, TweenTarget::Float(0.0));
        assert_eq!(end, TweenTarget::Float(10.0));
    }

    #[test]
    fn track_value_at_midpoint() {
        let track = float_track("x", 0.0, 10.0, 1.0);
        let mid = track.value_at(0.5).unwrap();
        if let TweenTarget::Float(v) = mid {
            assert!((v - 5.0).abs() < 1e-9);
        } else {
            panic!("Expected Float");
        }
    }

    #[test]
    fn color_lerp() {
        let a = TweenTarget::Color([0, 0, 0]);
        let b = TweenTarget::Color([100, 200, 50]);
        let mid = AnimTrack::lerp_tween(&a, &b, 0.5);
        assert_eq!(mid, TweenTarget::Color([50, 100, 25]));
    }

    #[test]
    fn clip_state_at_returns_all_tracks() {
        let mut clip = AnimClip::new("test", 2.0, false);
        clip.add_track(float_track("alpha", 0.0, 1.0, 2.0));
        clip.add_track(float_track("scale", 1.0, 2.0, 2.0));
        let state = clip.state_at(1.0);
        assert!(state.contains_key("alpha"));
        assert!(state.contains_key("scale"));
    }

    #[test]
    fn animation_engine_metadata() {
        let mut engine = AnimationEngine::new(24.0, 320, 240);
        engine.add_clip(AnimClip::new("intro", 3.0, false));
        let meta = engine.frames_to_gif_metadata("intro");
        assert_eq!(meta.width, 320);
        assert_eq!(meta.height, 240);
        assert_eq!(meta.frame_count, 72); // 24fps * 3s
    }

    #[test]
    fn frame_to_ascii_produces_rows() {
        let frame: Vec<Vec<[u8; 3]>> = vec![
            vec![[0, 0, 0], [255, 255, 255]],
            vec![[128, 128, 128], [64, 64, 64]],
        ];
        let ascii = AnimationEngine::frame_to_ascii(&frame);
        let lines: Vec<&str> = ascii.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn clip_looping_wraps_time() {
        let mut clip = AnimClip::new("loop", 1.0, true);
        clip.add_track(float_track("v", 0.0, 1.0, 1.0));
        let s0 = clip.state_at(0.0);
        let s1 = clip.state_at(1.0); // same as 0.0 with looping
                                     // At t=1.0 mod 1.0 = 0.0 → value should be 0.0
        assert_eq!(s0.get("v"), s1.get("v"));
    }
}
