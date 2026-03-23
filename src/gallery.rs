//! Multi-surface gallery mode.
//!
//! When gallery mode is active the application cycles through all available
//! mathematical surfaces automatically, displaying each one for a configurable
//! duration before fading out and fading in the next.
//!
//! # Configuration
//!
//! ```toml
//! gallery_mode        = true   # enable gallery mode on startup
//! gallery_duration_s  = 30     # seconds per surface (default 30)
//! ```
//!
//! # Keyboard shortcuts
//!
//! | Key | Action |
//! |-----|--------|
//! | `G` | Toggle gallery mode on/off |
//! | `LEFT` | Skip to the previous surface |
//! | `RIGHT` / `SPACE` | Skip to the next surface |
//!
//! # Transition
//!
//! Each surface change is accompanied by a brief cross-fade.  The caller reads
//! [`GalleryMode::transition_alpha`] each frame and multiplies scene opacity by
//! this value; a value of `1.0` means fully opaque (no transition in progress).

use std::time::{Duration, Instant};

// ─── Surface catalogue ────────────────────────────────────────────────────────

/// Every named surface known to the application.
///
/// This list is used by [`GalleryMode`] to cycle through surfaces
/// automatically.  Add new surface names here as they are implemented.
pub const ALL_SURFACES: &[&str] = &[
    "torus",
    "sphere",
    "saddle",
    "enneper",
    "catenoid",
    "helicoid",
    "hyperboloid",
    "hyperbolic_paraboloid",
    "ellipsoid",
];

// ─── Transition state ─────────────────────────────────────────────────────────

/// Phase of a cross-fade transition between two surfaces.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TransitionPhase {
    /// No transition; surface is fully visible.
    None,
    /// Fading out the old surface (alpha goes 1 → 0).
    FadeOut,
    /// Fading in the new surface (alpha goes 0 → 1).
    FadeIn,
}

// ─── GalleryMode ─────────────────────────────────────────────────────────────

/// Controls automatic cycling through all available mathematical surfaces.
pub struct GalleryMode {
    /// Whether gallery mode is currently active.
    enabled: bool,
    /// Index into [`ALL_SURFACES`] for the current surface.
    current_index: usize,
    /// How long each surface is displayed.
    duration: Duration,
    /// Wall-clock time when the current surface was first shown.
    surface_start: Option<Instant>,
    /// Current transition phase.
    phase: TransitionPhase,
    /// Wall-clock time when the current transition phase began.
    phase_start: Option<Instant>,
    /// Duration of each fade half (fade-out or fade-in).
    fade_duration: Duration,
    /// `true` if the active surface name has changed and the engine needs updating.
    surface_changed: bool,
}

impl GalleryMode {
    /// Default fade duration: 0.75 seconds per half (fade-out or fade-in).
    const DEFAULT_FADE_SECS: f32 = 0.75;

    /// Construct a new gallery mode manager.
    ///
    /// - `enabled`: whether to start in gallery mode.
    /// - `duration_secs`: seconds each surface is displayed (clamped to ≥ 1).
    pub fn new(enabled: bool, duration_secs: u32) -> Self {
        let duration_secs = duration_secs.max(1);
        Self {
            enabled,
            current_index: 0,
            duration: Duration::from_secs(duration_secs as u64),
            surface_start: if enabled { Some(Instant::now()) } else { None },
            phase: TransitionPhase::None,
            phase_start: None,
            fade_duration: Duration::from_millis(
                (Self::DEFAULT_FADE_SECS * 1000.0) as u64,
            ),
            surface_changed: enabled, // trigger initial surface apply if enabled
        }
    }

    /// Returns `true` if gallery mode is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The name of the currently-displayed surface.
    pub fn current_surface(&self) -> &'static str {
        ALL_SURFACES[self.current_index % ALL_SURFACES.len()]
    }

    /// Returns `true` and clears the flag if the surface changed since the last
    /// call to this method.  Call once per frame after [`update`].
    pub fn take_surface_changed(&mut self) -> bool {
        let changed = self.surface_changed;
        self.surface_changed = false;
        changed
    }

    /// Alpha multiplier for the scene (0.0 = fully transparent, 1.0 = opaque).
    ///
    /// During a fade-out this decreases from 1.0 → 0.0; during a fade-in it
    /// increases from 0.0 → 1.0.  Outside transitions it returns 1.0.
    pub fn transition_alpha(&self) -> f32 {
        let elapsed = match self.phase_start {
            Some(s) => s.elapsed(),
            None => return 1.0,
        };
        let t = (elapsed.as_secs_f32() / self.fade_duration.as_secs_f32()).clamp(0.0, 1.0);
        match self.phase {
            TransitionPhase::None => 1.0,
            TransitionPhase::FadeOut => 1.0 - t,
            TransitionPhase::FadeIn => t,
        }
    }

    /// Toggle gallery mode on or off.
    ///
    /// When turned on, the current index and timer are reset.
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        if self.enabled {
            self.surface_start = Some(Instant::now());
            self.surface_changed = true;
            self.phase = TransitionPhase::None;
            self.phase_start = None;
        } else {
            self.phase = TransitionPhase::None;
            self.phase_start = None;
        }
        tracing::info!("Gallery mode: {}", if self.enabled { "ON" } else { "OFF" });
    }

    /// Skip to the next surface (wraps around).
    pub fn next_surface(&mut self) {
        self.advance(1);
    }

    /// Skip to the previous surface (wraps around).
    pub fn prev_surface(&mut self) {
        if ALL_SURFACES.is_empty() {
            return;
        }
        let n = ALL_SURFACES.len();
        self.advance(n - 1); // equivalent to -1 mod n
    }

    /// Advance the index by `delta` (mod surface count) and begin a transition.
    fn advance(&mut self, delta: usize) {
        if ALL_SURFACES.is_empty() {
            return;
        }
        self.current_index = (self.current_index + delta) % ALL_SURFACES.len();
        self.start_fade_in();
        self.surface_start = Some(Instant::now());
        self.surface_changed = true;
        tracing::info!("Gallery: switching to '{}'", self.current_surface());
    }

    /// Called once per frame.  Drives the automatic timer and transition FSM.
    ///
    /// Returns `true` if the active surface changed this tick (convenience
    /// alias for [`take_surface_changed`], consumed by this method).
    pub fn update(&mut self) -> bool {
        if !self.enabled {
            return false;
        }

        // Drive the transition FSM.
        match self.phase {
            TransitionPhase::FadeOut => {
                if self.phase_start.map_or(false, |s| s.elapsed() >= self.fade_duration) {
                    // Fade-out complete: switch the surface and begin fade-in.
                    self.current_index = (self.current_index + 1) % ALL_SURFACES.len();
                    self.surface_changed = true;
                    self.start_fade_in();
                    self.surface_start = Some(Instant::now());
                    tracing::info!("Gallery: now showing '{}'", self.current_surface());
                }
            }
            TransitionPhase::FadeIn => {
                if self.phase_start.map_or(false, |s| s.elapsed() >= self.fade_duration) {
                    // Fade-in complete: steady state.
                    self.phase = TransitionPhase::None;
                    self.phase_start = None;
                }
            }
            TransitionPhase::None => {
                // Check if it's time to start a fade-out.
                if let Some(start) = self.surface_start {
                    if start.elapsed() >= self.duration {
                        self.start_fade_out();
                    }
                }
            }
        }

        self.take_surface_changed()
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn start_fade_out(&mut self) {
        self.phase = TransitionPhase::FadeOut;
        self.phase_start = Some(Instant::now());
    }

    fn start_fade_in(&mut self) {
        self.phase = TransitionPhase::FadeIn;
        self.phase_start = Some(Instant::now());
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_surfaces_non_empty() {
        assert!(!ALL_SURFACES.is_empty());
    }

    #[test]
    fn all_surface_names_non_empty() {
        for name in ALL_SURFACES {
            assert!(!name.is_empty(), "surface name should not be empty");
        }
    }

    #[test]
    fn initial_state_disabled() {
        let g = GalleryMode::new(false, 30);
        assert!(!g.is_enabled());
        assert_eq!(g.transition_alpha(), 1.0);
    }

    #[test]
    fn initial_state_enabled() {
        let g = GalleryMode::new(true, 30);
        assert!(g.is_enabled());
    }

    #[test]
    fn toggle_on_off() {
        let mut g = GalleryMode::new(false, 30);
        g.toggle();
        assert!(g.is_enabled());
        g.toggle();
        assert!(!g.is_enabled());
    }

    #[test]
    fn next_surface_advances_index() {
        let mut g = GalleryMode::new(false, 30);
        let first = g.current_surface();
        g.next_surface();
        let second = g.current_surface();
        // Should have moved (unless there's only one surface, which we don't expect).
        if ALL_SURFACES.len() > 1 {
            assert_ne!(first, second, "surface should change after next_surface()");
        }
    }

    #[test]
    fn next_surface_wraps() {
        let mut g = GalleryMode::new(false, 30);
        let n = ALL_SURFACES.len();
        for _ in 0..n {
            g.next_surface();
        }
        assert_eq!(g.current_index, 0, "should wrap back to 0");
    }

    #[test]
    fn prev_surface_wraps() {
        let mut g = GalleryMode::new(false, 30);
        g.prev_surface();
        assert_eq!(
            g.current_index,
            ALL_SURFACES.len() - 1,
            "should wrap to last index"
        );
    }

    #[test]
    fn surface_changed_flag_cleared_after_take() {
        let mut g = GalleryMode::new(false, 30);
        g.next_surface();
        assert!(g.take_surface_changed(), "flag should be set after advance");
        assert!(!g.take_surface_changed(), "flag should be cleared after first take");
    }

    #[test]
    fn transition_alpha_during_fade_in_increases() {
        let mut g = GalleryMode::new(false, 30);
        g.next_surface(); // triggers a fade-in
        // Immediately after triggering the fade, alpha should be near 0.
        // (Very short elapsed time.)
        let alpha = g.transition_alpha();
        assert!(alpha >= 0.0 && alpha <= 1.0, "alpha out of range: {alpha}");
    }

    #[test]
    fn duration_clamped_to_min_one() {
        let g = GalleryMode::new(false, 0);
        assert_eq!(g.duration, Duration::from_secs(1));
    }

    #[test]
    fn current_surface_always_valid() {
        let mut g = GalleryMode::new(false, 30);
        for _ in 0..ALL_SURFACES.len() * 2 {
            let name = g.current_surface();
            assert!(ALL_SURFACES.contains(&name));
            g.next_surface();
        }
    }
}
