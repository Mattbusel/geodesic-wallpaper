//! Interactive geodesic shooting via mouse input.
//!
//! When the user clicks on the wallpaper window a new geodesic is shot from
//! the surface point nearest to the mouse cursor.  The module provides:
//!
//! - [`GeodesicShooter`]: translates raw Win32 mouse coordinates into surface
//!   parameter coordinates and constructs a new [`Geodesic`].
//! - [`MouseEvent`]: high-level mouse events extracted from Win32 messages.
//! - Helper functions for inverse parameterization (screen → surface).
//!
//! # Mouse bindings
//!
//! | Action | Effect |
//! |--------|--------|
//! | Left-click | Shoot a new geodesic from the clicked surface point |
//! | Right-click | Remove all geodesics and reset to initial configuration |
//! | Middle-click | Cycle to the next surface type |
//! | Scroll up | Increase geodesic speed (multiply by 1.1) |
//! | Scroll down | Decrease geodesic speed (multiply by 0.9) |
//!
//! # Inverse parameterization
//!
//! Mapping a screen pixel to surface parameters is done by ray-casting against
//! a sampled grid of the surface and returning the `(u, v)` of the closest
//! sample point.  This is not exact but is fast enough for interactive use and
//! handles all surfaces (including self-intersecting ones) uniformly.

use crate::geodesic::Geodesic;
use crate::surface::Surface;

// ---------------------------------------------------------------------------
// Mouse event type
// ---------------------------------------------------------------------------

/// High-level mouse events produced by the Win32 message loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MouseEvent {
    /// Left mouse button pressed at `(x, y)` in screen pixels.
    LeftClick { x: i32, y: i32 },
    /// Right mouse button pressed — reset all geodesics.
    RightClick,
    /// Middle mouse button pressed — cycle to next surface.
    MiddleClick,
    /// Scroll wheel moved; `delta` is positive for scroll-up (speed increase).
    Scroll { delta: f32 },
}

// ---------------------------------------------------------------------------
// Speed controller
// ---------------------------------------------------------------------------

/// Tracks a global geodesic speed multiplier adjusted by the scroll wheel.
///
/// The multiplier is applied to all newly spawned geodesics as a scale on
/// their initial velocity magnitude.  It does **not** retroactively affect
/// already-running geodesics.
#[derive(Debug, Clone)]
pub struct SpeedController {
    /// Current speed multiplier (default: 1.0).
    pub multiplier: f32,
    /// Minimum multiplier (default: 0.1).
    pub min: f32,
    /// Maximum multiplier (default: 10.0).
    pub max: f32,
    /// Factor applied per scroll click (default: 1.1).
    pub step: f32,
}

impl Default for SpeedController {
    fn default() -> Self {
        Self {
            multiplier: 1.0,
            min: 0.1,
            max: 10.0,
            step: 1.1,
        }
    }
}

impl SpeedController {
    /// Apply a scroll delta (positive = speed up, negative = slow down).
    pub fn apply_scroll(&mut self, delta: f32) {
        if delta > 0.0 {
            self.multiplier = (self.multiplier * self.step).min(self.max);
        } else if delta < 0.0 {
            self.multiplier = (self.multiplier / self.step).max(self.min);
        }
    }

    /// Reset to the default speed multiplier.
    pub fn reset(&mut self) {
        self.multiplier = 1.0;
    }
}

// ---------------------------------------------------------------------------
// Inverse parameterization
// ---------------------------------------------------------------------------

/// Find the surface parameters `(u, v)` closest to the 3-D ray implied by a
/// screen click.
///
/// The approach samples a uniform grid of `grid_u × grid_v` points on the
/// surface, projects each to screen space using the provided projection-view
/// matrix, and returns the `(u, v)` with the smallest Euclidean distance to
/// the click.
///
/// # Parameters
/// - `surface` — the surface to query.
/// - `mvp` — 4×4 column-major projection-view-model matrix as `[f32; 16]`.
/// - `ndc_x`, `ndc_y` — normalised device coordinates in `[-1, 1]` of the
///   mouse click (x left→right, y bottom→top in standard NDC).
/// - `grid_u`, `grid_v` — number of grid samples per axis.
///
/// Returns `None` if the surface grid produces no finite screen-space points.
pub fn inverse_parameterize(
    surface: &dyn Surface,
    mvp: &[f32; 16],
    ndc_x: f32,
    ndc_y: f32,
    grid_u: u32,
    grid_v: u32,
) -> Option<(f32, f32)> {
    use std::f32::consts::TAU;

    let mut best_dist2 = f32::MAX;
    let mut best_uv: Option<(f32, f32)> = None;

    for ui in 0..grid_u {
        for vi in 0..grid_v {
            let u = (ui as f32 / grid_u as f32) * TAU;
            let v = (vi as f32 / grid_v as f32) * TAU;
            let p = surface.position(u, v);

            // Transform to clip space: clip = MVP * [x, y, z, 1]^T
            let cx = mvp[0] * p.x + mvp[4] * p.y + mvp[8] * p.z + mvp[12];
            let cy = mvp[1] * p.x + mvp[5] * p.y + mvp[9] * p.z + mvp[13];
            let _cz = mvp[2] * p.x + mvp[6] * p.y + mvp[10] * p.z + mvp[14];
            let cw = mvp[3] * p.x + mvp[7] * p.y + mvp[11] * p.z + mvp[15];

            if cw.abs() < 1e-6 || !cw.is_finite() {
                continue;
            }

            // Perspective divide → NDC.
            let sx = cx / cw;
            let sy = cy / cw;

            if !sx.is_finite() || !sy.is_finite() {
                continue;
            }

            // Only consider points on the near side of the camera.
            if cw < 0.0 {
                continue;
            }

            let dx = sx - ndc_x;
            let dy = sy - ndc_y;
            let dist2 = dx * dx + dy * dy;

            if dist2 < best_dist2 {
                best_dist2 = dist2;
                best_uv = Some((u, v));
            }
        }
    }

    best_uv
}

/// Convert a Win32 screen-pixel coordinate to NDC given window dimensions.
///
/// Win32 `y` is top-down (0 at top); NDC `y` is bottom-up (-1 at bottom, +1
/// at top).  Both `ndc_x` and `ndc_y` are in `[-1, 1]`.
pub fn screen_to_ndc(px: i32, py: i32, width: u32, height: u32) -> (f32, f32) {
    let ndc_x = (px as f32 / width as f32) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py as f32 / height as f32) * 2.0;
    (ndc_x, ndc_y)
}

// ---------------------------------------------------------------------------
// GeodesicShooter
// ---------------------------------------------------------------------------

/// Translates mouse events into new geodesics or control commands.
///
/// # Example (conceptual)
///
/// ```no_run
/// use geodesic_wallpaper::interactive::{GeodesicShooter, MouseEvent};
///
/// let mut shooter = GeodesicShooter::new(1920, 1080, 30, 300);
///
/// // In the Win32 message loop:
/// // let event = MouseEvent::LeftClick { x: 640, y: 400 };
/// // shooter.handle(event, surface.as_ref(), &mvp, &mut geodesics, &mut trails, &mut rng);
/// ```
pub struct GeodesicShooter {
    /// Window width in pixels.
    pub width: u32,
    /// Window height in pixels.
    pub height: u32,
    /// Max age (in frames) for newly shot geodesics.
    pub max_age: usize,
    /// Colour palette index for the next shot geodesic (cycles through).
    color_idx: usize,
    /// Total number of palette colours available.
    num_colors: usize,
    /// Grid resolution for inverse parameterization (u direction).
    pub grid_u: u32,
    /// Grid resolution for inverse parameterization (v direction).
    pub grid_v: u32,
    /// Speed multiplier for newly shot geodesics.
    pub speed: SpeedController,
    /// Pending command to cycle surface (consumed by the main loop).
    pending_cycle_surface: bool,
    /// Pending command to reset all geodesics (consumed by the main loop).
    pending_reset: bool,
}

impl GeodesicShooter {
    /// Construct a new geodesic shooter.
    ///
    /// - `width`, `height` — wallpaper window dimensions.
    /// - `max_age` — lifetime in frames for newly shot geodesics.
    /// - `num_colors` — size of the colour palette (for cycling).
    pub fn new(width: u32, height: u32, max_age: usize, num_colors: usize) -> Self {
        Self {
            width,
            height,
            max_age,
            color_idx: 0,
            num_colors: num_colors.max(1),
            grid_u: 64,
            grid_v: 64,
            speed: SpeedController::default(),
            pending_cycle_surface: false,
            pending_reset: false,
        }
    }

    /// Handle a mouse event.
    ///
    /// Returns `Some(Geodesic)` when a new geodesic should be added (left-click
    /// that successfully hits the surface), `None` otherwise.  Check
    /// [`pending_reset()`][Self::pending_reset] and
    /// [`pending_cycle_surface()`][Self::pending_cycle_surface] after each call.
    pub fn handle(
        &mut self,
        event: MouseEvent,
        surface: &dyn Surface,
        mvp: &[f32; 16],
        rng: &mut dyn rand::RngCore,
    ) -> Option<Geodesic> {
        match event {
            MouseEvent::LeftClick { x, y } => {
                let (ndc_x, ndc_y) = screen_to_ndc(x, y, self.width, self.height);
                let uv =
                    inverse_parameterize(surface, mvp, ndc_x, ndc_y, self.grid_u, self.grid_v)?;
                let (u, v) = surface.wrap(uv.0, uv.1);
                let (du, dv) = surface.random_tangent(u, v, rng);
                let du = du * self.speed.multiplier;
                let dv = dv * self.speed.multiplier;
                let ci = self.color_idx;
                self.color_idx = (self.color_idx + 1) % self.num_colors;
                Some(Geodesic::new(u, v, du, dv, self.max_age, ci))
            }
            MouseEvent::RightClick => {
                self.pending_reset = true;
                self.speed.reset();
                None
            }
            MouseEvent::MiddleClick => {
                self.pending_cycle_surface = true;
                None
            }
            MouseEvent::Scroll { delta } => {
                self.speed.apply_scroll(delta);
                None
            }
        }
    }

    /// Returns `true` and clears the flag if a reset was requested (right-click).
    pub fn pending_reset(&mut self) -> bool {
        let v = self.pending_reset;
        self.pending_reset = false;
        v
    }

    /// Returns `true` and clears the flag if a surface cycle was requested (middle-click).
    pub fn pending_cycle_surface(&mut self) -> bool {
        let v = self.pending_cycle_surface;
        self.pending_cycle_surface = false;
        v
    }

    /// Update the window dimensions (e.g. on resize).
    pub fn set_dimensions(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::torus::Torus;
    use rand::SeedableRng;

    fn torus() -> Torus {
        Torus::new(2.0, 0.7)
    }

    fn identity_mvp() -> [f32; 16] {
        [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ]
    }

    #[test]
    fn screen_to_ndc_center() {
        let (nx, ny) = screen_to_ndc(960, 540, 1920, 1080);
        assert!((nx - 0.0).abs() < 0.01, "ndc_x center: {nx}");
        assert!((ny - 0.0).abs() < 0.01, "ndc_y center: {ny}");
    }

    #[test]
    fn screen_to_ndc_top_left() {
        let (nx, ny) = screen_to_ndc(0, 0, 1920, 1080);
        assert!((nx - (-1.0)).abs() < 0.01, "ndc_x top-left: {nx}");
        assert!((ny - 1.0).abs() < 0.01, "ndc_y top-left: {ny}");
    }

    #[test]
    fn inverse_parameterize_returns_some_for_identity_mvp() {
        // With identity MVP the torus at (u=0, v=0) maps to (R+r, 0) ≈ (2.7, 0)
        // in clip space. With NDC near that point some sample should be found.
        let t = torus();
        // Try a click near the origin in NDC — some sample will always be closest.
        let result = inverse_parameterize(&t, &identity_mvp(), 0.0, 0.0, 32, 32);
        assert!(result.is_some(), "expected Some(u,v)");
    }

    #[test]
    fn shooter_left_click_spawns_geodesic() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut shooter = GeodesicShooter::new(1920, 1080, 300, 5);
        let t = torus();
        let mvp = identity_mvp();
        let geo = shooter.handle(MouseEvent::LeftClick { x: 960, y: 540 }, &t, &mvp, &mut rng);
        assert!(geo.is_some(), "expected a geodesic on left-click");
    }

    #[test]
    fn shooter_right_click_sets_reset_flag() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut shooter = GeodesicShooter::new(1920, 1080, 300, 5);
        let t = torus();
        let mvp = identity_mvp();
        shooter.handle(MouseEvent::RightClick, &t, &mvp, &mut rng);
        assert!(shooter.pending_reset(), "expected reset flag");
        assert!(
            !shooter.pending_reset(),
            "flag should be cleared after read"
        );
    }

    #[test]
    fn shooter_middle_click_sets_cycle_flag() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let mut shooter = GeodesicShooter::new(1920, 1080, 300, 5);
        let t = torus();
        let mvp = identity_mvp();
        shooter.handle(MouseEvent::MiddleClick, &t, &mvp, &mut rng);
        assert!(shooter.pending_cycle_surface(), "expected cycle flag");
        assert!(!shooter.pending_cycle_surface(), "flag should be cleared");
    }

    #[test]
    fn scroll_up_increases_speed() {
        let mut ctrl = SpeedController::default();
        let before = ctrl.multiplier;
        ctrl.apply_scroll(1.0);
        assert!(ctrl.multiplier > before, "scroll up should increase speed");
    }

    #[test]
    fn scroll_down_decreases_speed() {
        let mut ctrl = SpeedController::default();
        let before = ctrl.multiplier;
        ctrl.apply_scroll(-1.0);
        assert!(
            ctrl.multiplier < before,
            "scroll down should decrease speed"
        );
    }

    #[test]
    fn speed_controller_clamped_at_limits() {
        let mut ctrl = SpeedController::default();
        for _ in 0..100 {
            ctrl.apply_scroll(1.0);
        }
        assert!(
            ctrl.multiplier <= ctrl.max,
            "speed should be clamped at max"
        );
        ctrl.reset();
        for _ in 0..100 {
            ctrl.apply_scroll(-1.0);
        }
        assert!(
            ctrl.multiplier >= ctrl.min,
            "speed should be clamped at min"
        );
    }

    #[test]
    fn color_cycles_through_palette() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let mut shooter = GeodesicShooter::new(100, 100, 300, 3);
        let t = torus();
        let mvp = identity_mvp();
        let mut color_indices = Vec::new();
        for _ in 0..6 {
            if let Some(g) =
                shooter.handle(MouseEvent::LeftClick { x: 50, y: 50 }, &t, &mvp, &mut rng)
            {
                color_indices.push(g.color_idx);
            }
        }
        // Should cycle 0,1,2,0,1,2
        if color_indices.len() >= 4 {
            assert_eq!(
                color_indices[0], color_indices[3],
                "color should cycle: {:?}",
                color_indices
            );
        }
    }
}
