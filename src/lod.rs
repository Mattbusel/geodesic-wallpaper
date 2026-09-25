//! Dynamic Level of Detail (LOD) controller.
//!
//! Monitors GPU frame time and automatically adjusts the geodesic count to
//! maintain a smooth 60 fps target.
//!
//! # Algorithm
//!
//! The controller uses a PI-style feedback loop on the measured frame time:
//!
//! - If `frame_time > 16 ms` (below 60 fps): reduce geodesic count by one step.
//! - If `frame_time < 8 ms` (above 125 fps): increase geodesic count by one step.
//! - Changes are hysteresis-gated: at least `cooldown_frames` must elapse
//!   between adjustments to avoid oscillation.
//!
//! The step size grows when the frame time is far from target (proportional
//! action) and shrinks close to target (fine-grained adjustment).

#![allow(dead_code)]

// ── Configuration ─────────────────────────────────────────────────────────────

/// Configuration for the LOD controller.
#[derive(Debug, Clone)]
pub struct LodConfig {
    /// Minimum number of geodesics (absolute floor).
    pub min_geodesics: usize,
    /// Maximum number of geodesics (absolute ceiling).
    pub max_geodesics: usize,
    /// Target frame time in milliseconds (default: 16.67 ms = 60 fps).
    pub target_ms: f32,
    /// Upper threshold: reduce geodesics when frame time exceeds this (ms).
    pub reduce_threshold_ms: f32,
    /// Lower threshold: increase geodesics when frame time is below this (ms).
    pub increase_threshold_ms: f32,
    /// Minimum frames between geodesic count changes (hysteresis).
    pub cooldown_frames: u32,
    /// Initial step size for geodesic count changes.
    pub base_step: usize,
    /// EMA coefficient for frame time smoothing (0 < alpha <= 1).
    pub smooth_alpha: f32,
}

impl Default for LodConfig {
    fn default() -> Self {
        Self {
            min_geodesics: 4,
            max_geodesics: 200,
            target_ms: 16.667,
            reduce_threshold_ms: 16.0,
            increase_threshold_ms: 8.0,
            cooldown_frames: 30,
            base_step: 2,
            smooth_alpha: 0.1,
        }
    }
}

// ── Controller ────────────────────────────────────────────────────────────────

/// Stateful LOD controller.
///
/// Call [`LodController::update`] once per frame with the measured frame time,
/// then read [`LodController::geodesic_count`] to get the recommended count.
pub struct LodController {
    pub config: LodConfig,
    /// Current recommended geodesic count.
    pub geodesic_count: usize,
    /// Exponentially smoothed frame time in ms.
    smoothed_ms: f32,
    /// Frames since last count change.
    cooldown_remaining: u32,
    /// Total frames processed.
    frame_count: u64,
    /// Running statistics for diagnostics.
    stats: LodStats,
}

/// Diagnostics emitted by the LOD controller.
#[derive(Debug, Clone, Default)]
pub struct LodStats {
    /// Total number of upward adjustments (geodesic count increased).
    pub increases: u32,
    /// Total number of downward adjustments (geodesic count decreased).
    pub decreases: u32,
    /// Minimum smoothed frame time observed (ms).
    pub min_frame_ms: f32,
    /// Maximum smoothed frame time observed (ms).
    pub max_frame_ms: f32,
    /// Current smoothed frame time (ms).
    pub current_frame_ms: f32,
}

/// The action taken by the LOD controller on a given frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LodAction {
    /// No change this frame.
    Unchanged,
    /// Geodesic count was increased.
    Increased { from: usize, to: usize },
    /// Geodesic count was decreased.
    Decreased { from: usize, to: usize },
}

impl LodController {
    /// Create a controller with the given initial geodesic count.
    pub fn new(initial_geodesics: usize, config: LodConfig) -> Self {
        let clamped = initial_geodesics.clamp(config.min_geodesics, config.max_geodesics);
        Self {
            geodesic_count: clamped,
            smoothed_ms: config.target_ms,
            cooldown_remaining: config.cooldown_frames,
            frame_count: 0,
            stats: LodStats {
                min_frame_ms: f32::MAX,
                max_frame_ms: 0.0,
                ..Default::default()
            },
            config,
        }
    }

    /// Feed the measured frame time (in milliseconds) for this frame.
    ///
    /// Returns the [`LodAction`] taken (or [`LodAction::Unchanged`]).
    pub fn update(&mut self, frame_ms: f32) -> LodAction {
        self.frame_count += 1;

        // EMA smoothing.
        let a = self.config.smooth_alpha;
        self.smoothed_ms = (1.0 - a) * self.smoothed_ms + a * frame_ms;

        // Update stats.
        self.stats.current_frame_ms = self.smoothed_ms;
        if self.smoothed_ms < self.stats.min_frame_ms {
            self.stats.min_frame_ms = self.smoothed_ms;
        }
        if self.smoothed_ms > self.stats.max_frame_ms {
            self.stats.max_frame_ms = self.smoothed_ms;
        }

        // Cooldown.
        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
            return LodAction::Unchanged;
        }

        let old_count = self.geodesic_count;

        if self.smoothed_ms > self.config.reduce_threshold_ms {
            // Too slow → reduce.
            let excess = self.smoothed_ms - self.config.target_ms;
            let step = (self.config.base_step + (excess / 4.0) as usize).min(20);
            let new_count = old_count
                .saturating_sub(step)
                .max(self.config.min_geodesics);
            if new_count < old_count {
                self.geodesic_count = new_count;
                self.cooldown_remaining = self.config.cooldown_frames;
                self.stats.decreases += 1;
                log::debug!(
                    "[lod] frame={:.1}ms → reduce {} → {} geodesics",
                    self.smoothed_ms,
                    old_count,
                    new_count
                );
                return LodAction::Decreased {
                    from: old_count,
                    to: new_count,
                };
            }
        } else if self.smoothed_ms < self.config.increase_threshold_ms {
            // Too fast → increase.
            let headroom = self.config.increase_threshold_ms - self.smoothed_ms;
            let step = (self.config.base_step + (headroom / 2.0) as usize).min(10);
            let new_count = (old_count + step).min(self.config.max_geodesics);
            if new_count > old_count {
                self.geodesic_count = new_count;
                self.cooldown_remaining = self.config.cooldown_frames;
                self.stats.increases += 1;
                log::debug!(
                    "[lod] frame={:.1}ms → increase {} → {} geodesics",
                    self.smoothed_ms,
                    old_count,
                    new_count
                );
                return LodAction::Increased {
                    from: old_count,
                    to: new_count,
                };
            }
        }

        LodAction::Unchanged
    }

    /// Return a snapshot of the controller's diagnostics.
    pub fn stats(&self) -> &LodStats {
        &self.stats
    }

    /// Smoothed frame time in ms.
    pub fn smoothed_frame_ms(&self) -> f32 {
        self.smoothed_ms
    }

    /// Number of frames processed so far.
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Force-set the geodesic count without going through the feedback loop.
    /// Useful when the user overrides the count manually.
    pub fn force_set(&mut self, count: usize) {
        self.geodesic_count = count.clamp(self.config.min_geodesics, self.config.max_geodesics);
        self.cooldown_remaining = self.config.cooldown_frames;
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_controller(initial: usize) -> LodController {
        LodController::new(
            initial,
            LodConfig {
                min_geodesics: 4,
                max_geodesics: 100,
                cooldown_frames: 0, // no cooldown for tests
                ..Default::default()
            },
        )
    }

    #[test]
    fn test_lod_reduces_when_slow() {
        let mut ctrl = make_controller(30);
        // Feed a very slow frame time repeatedly.
        let mut decreased = false;
        for _ in 0..20 {
            let action = ctrl.update(33.0); // 30 fps
            decreased |= matches!(action, LodAction::Decreased { .. });
        }
        assert!(decreased, "should reduce geodesics on slow frame");
        assert!(ctrl.geodesic_count < 30, "geodesic count should decrease");
    }

    #[test]
    fn test_lod_increases_when_fast() {
        let mut ctrl = make_controller(30);
        let mut action = LodAction::Unchanged;
        for _ in 0..20 {
            action = ctrl.update(3.0); // very fast
        }
        assert!(
            matches!(action, LodAction::Increased { .. }),
            "should increase geodesics on fast frame: {:?}",
            action
        );
        assert!(ctrl.geodesic_count > 30, "geodesic count should increase");
    }

    #[test]
    fn test_lod_unchanged_at_target() {
        let mut ctrl = make_controller(30);
        let mut last_action = LodAction::Unchanged;
        for _ in 0..50 {
            last_action = ctrl.update(12.0); // between 8 and 16 ms → stable
        }
        assert_eq!(
            last_action,
            LodAction::Unchanged,
            "should be stable at target"
        );
    }

    #[test]
    fn test_lod_clamps_to_min() {
        let mut ctrl = make_controller(5);
        for _ in 0..100 {
            ctrl.update(100.0); // extremely slow
        }
        assert!(ctrl.geodesic_count >= ctrl.config.min_geodesics);
    }

    #[test]
    fn test_lod_clamps_to_max() {
        let mut ctrl = make_controller(90);
        for _ in 0..100 {
            ctrl.update(0.1); // extremely fast
        }
        assert!(ctrl.geodesic_count <= ctrl.config.max_geodesics);
    }

    #[test]
    fn test_lod_force_set() {
        let mut ctrl = make_controller(30);
        ctrl.force_set(50);
        assert_eq!(ctrl.geodesic_count, 50);
    }

    #[test]
    fn test_lod_stats_tracked() {
        let mut ctrl = make_controller(30);
        ctrl.update(20.0);
        ctrl.update(10.0);
        assert!(ctrl.stats().max_frame_ms >= ctrl.stats().min_frame_ms);
    }
}
