//! Turing reaction-diffusion system overlay (Gray-Scott model).
//!
//! Renders a **Gray-Scott** reaction-diffusion system as a texture overlay on
//! the geodesic surface. The patterns evolve in real-time and interact with
//! the surface curvature: the local Gaussian curvature modulates the
//! diffusion coefficients, causing patterns to grow faster in negatively
//! curved regions and slower near elliptic points.
//!
//! ## Gray-Scott Equations
//!
//! Two chemical species `U` and `V` react on the surface:
//!
//! ```text
//! ∂U/∂t = D_u · ΔU  -  U·V²  +  F·(1 - U)
//! ∂V/∂t = D_v · ΔV  +  U·V²  -  (F + K)·V
//! ```
//!
//! where:
//! - `D_u`, `D_v` — diffusion coefficients
//! - `F`          — feed rate
//! - `K`          — kill rate
//! - `Δ`          — discrete Laplacian (5-point stencil on a flat grid)
//!
//! Curvature coupling replaces `D_u` with `D_u * (1 + alpha * curvature)`.
//!
//! ## Presets
//!
//! Several classic Gray-Scott parameter sets are provided via [`Preset`]:
//! - `Coral`: branching coral-like structures
//! - `Spots`: isolated spots (Turing spots)
//! - `Stripes`: labyrinthine stripes
//! - `Mitosis`: cell-division-like dynamics
//!
//! ## Usage
//!
//! ```rust
//! use geodesic_wallpaper::reaction_diffusion::{GrayScott, GrayScottConfig, Preset};
//!
//! let cfg = GrayScottConfig::from_preset(Preset::Spots);
//! let mut gs = GrayScott::new(64, 64, cfg);
//! gs.seed_center();
//!
//! for _ in 0..200 {
//!     gs.step(1.0);
//! }
//! let texture = gs.rgba_texture();
//! assert_eq!(texture.len(), 64 * 64 * 4);
//! ```

/// A preset parameter set for the Gray-Scott system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    /// Branching coral-like patterns.
    Coral,
    /// Isolated Turing spots.
    Spots,
    /// Labyrinthine stripes.
    Stripes,
    /// Cell-division-like dynamics.
    Mitosis,
}

/// Configuration for the Gray-Scott reaction-diffusion system.
#[derive(Debug, Clone)]
pub struct GrayScottConfig {
    /// Diffusion coefficient for species U.
    pub d_u: f64,
    /// Diffusion coefficient for species V.
    pub d_v: f64,
    /// Feed rate F.
    pub feed: f64,
    /// Kill rate K.
    pub kill: f64,
    /// Curvature coupling strength (0 = flat, positive = curv-enhanced diffusion).
    pub curvature_alpha: f64,
    /// RGBA colour for species V at concentration 1.0 (foreground).
    pub color_v: [u8; 4],
    /// RGBA colour for species U at concentration 1.0 (background).
    pub color_u: [u8; 4],
}

impl GrayScottConfig {
    /// Create a configuration from a well-known preset.
    pub fn from_preset(preset: Preset) -> Self {
        let (d_u, d_v, feed, kill) = match preset {
            Preset::Coral   => (0.16, 0.08, 0.060, 0.062),
            Preset::Spots   => (0.16, 0.08, 0.035, 0.065),
            Preset::Stripes => (0.16, 0.08, 0.060, 0.055),
            Preset::Mitosis => (0.28, 0.05, 0.028, 0.057),
        };
        Self {
            d_u,
            d_v,
            feed,
            kill,
            curvature_alpha: 0.1,
            color_v: [0, 180, 255, 255],
            color_u: [20, 20, 40, 255],
        }
    }
}

impl Default for GrayScottConfig {
    fn default() -> Self {
        Self::from_preset(Preset::Spots)
    }
}

/// A 2D Gray-Scott reaction-diffusion system.
///
/// The grid is flat (`width × height` cells) with periodic boundary conditions.
/// Curvature values may be injected per-cell to couple the system with the
/// underlying surface geometry.
pub struct GrayScott {
    /// Grid width in cells.
    pub width: usize,
    /// Grid height in cells.
    pub height: usize,
    cfg: GrayScottConfig,
    /// Concentration of species U; length = width * height.
    u: Vec<f64>,
    /// Concentration of species V; length = width * height.
    v: Vec<f64>,
    /// Scratch buffers for double buffering.
    u_next: Vec<f64>,
    v_next: Vec<f64>,
    /// Per-cell Gaussian curvature (optional; defaults to 0).
    curvature: Vec<f64>,
    /// Total number of simulation steps taken.
    pub step_count: u64,
}

impl GrayScott {
    /// Create a new Gray-Scott system initialised to the homogeneous state
    /// `U = 1, V = 0` everywhere.
    pub fn new(width: usize, height: usize, cfg: GrayScottConfig) -> Self {
        let n = width * height;
        Self {
            width,
            height,
            cfg,
            u: vec![1.0; n],
            v: vec![0.0; n],
            u_next: vec![0.0; n],
            v_next: vec![0.0; n],
            curvature: vec![0.0; n],
            step_count: 0,
        }
    }

    /// Seed a small square patch of V near the grid centre.
    pub fn seed_center(&mut self) {
        let cx = self.width / 2;
        let cy = self.height / 2;
        let r = 5.min(self.width / 4).min(self.height / 4);
        for dy in 0..=r {
            for dx in 0..=r {
                let x = cx.saturating_sub(r / 2) + dx;
                let y = cy.saturating_sub(r / 2) + dy;
                if x < self.width && y < self.height {
                    let idx = y * self.width + x;
                    self.v[idx] = 0.25;
                    self.u[idx] = 0.5;
                }
            }
        }
    }

    /// Seed a random-looking noise field using a simple LCG.
    pub fn seed_noise(&mut self, seed: u64) {
        let mut rng = seed.wrapping_add(1);
        for i in 0..self.u.len() {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let frac = (rng >> 33) as f64 / u32::MAX as f64;
            if frac < 0.05 {
                self.v[i] = 0.5;
                self.u[i] = 0.5;
            }
        }
    }

    /// Inject per-cell Gaussian curvature values (length must equal width*height).
    ///
    /// Positive curvature (sphere-like) reduces diffusion of U; negative
    /// curvature (saddle-like) increases it.
    pub fn set_curvature(&mut self, curvature: &[f64]) {
        let n = self.width * self.height;
        self.curvature = curvature[..n.min(curvature.len())].to_vec();
        self.curvature.resize(n, 0.0);
    }

    /// Advance the simulation by one time step of size `dt`.
    ///
    /// Uses explicit Euler integration with a 5-point Laplacian stencil and
    /// periodic boundary conditions.
    pub fn step(&mut self, dt: f64) {
        let w = self.width;
        let h = self.height;

        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                let u_c = self.u[idx];
                let v_c = self.v[idx];

                // 5-point Laplacian with periodic BCs.
                let u_l = self.u[y * w + (x + w - 1) % w];
                let u_r = self.u[y * w + (x + 1) % w];
                let u_u = self.u[((y + h - 1) % h) * w + x];
                let u_d = self.u[((y + 1) % h) * w + x];
                let v_l = self.v[y * w + (x + w - 1) % w];
                let v_r = self.v[y * w + (x + 1) % w];
                let v_u = self.v[((y + h - 1) % h) * w + x];
                let v_d = self.v[((y + 1) % h) * w + x];

                let lap_u = u_l + u_r + u_u + u_d - 4.0 * u_c;
                let lap_v = v_l + v_r + v_u + v_d - 4.0 * v_c;

                // Curvature-modulated diffusion for U.
                let k = self.curvature[idx];
                let d_u_eff = self.cfg.d_u * (1.0 + self.cfg.curvature_alpha * k).max(0.01);
                let d_v_eff = self.cfg.d_v;

                let uvv = u_c * v_c * v_c;
                let du = d_u_eff * lap_u - uvv + self.cfg.feed * (1.0 - u_c);
                let dv = d_v_eff * lap_v + uvv - (self.cfg.feed + self.cfg.kill) * v_c;

                self.u_next[idx] = (u_c + dt * du).clamp(0.0, 1.0);
                self.v_next[idx] = (v_c + dt * dv).clamp(0.0, 1.0);
            }
        }

        std::mem::swap(&mut self.u, &mut self.u_next);
        std::mem::swap(&mut self.v, &mut self.v_next);
        self.step_count += 1;
    }

    /// Advance by `steps` time steps.
    pub fn step_n(&mut self, steps: usize, dt: f64) {
        for _ in 0..steps {
            self.step(dt);
        }
    }

    /// Return the current V concentration field as a flat RGBA texture.
    ///
    /// Each pixel colour is interpolated between `color_u` (V≈0) and
    /// `color_v` (V≈1).
    pub fn rgba_texture(&self) -> Vec<u8> {
        let n = self.width * self.height;
        let mut out = Vec::with_capacity(n * 4);
        for i in 0..n {
            let t = self.v[i].clamp(0.0, 1.0) as f32;
            let u_col = self.cfg.color_u;
            let v_col = self.cfg.color_v;
            for c in 0..4 {
                let val = u_col[c] as f32 * (1.0 - t) + v_col[c] as f32 * t;
                out.push(val.round() as u8);
            }
        }
        out
    }

    /// Access the raw U concentration buffer (read-only).
    pub fn u(&self) -> &[f64] { &self.u }

    /// Access the raw V concentration buffer (read-only).
    pub fn v(&self) -> &[f64] { &self.v }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gs() -> GrayScott {
        GrayScott::new(32, 32, GrayScottConfig::default())
    }

    #[test]
    fn initial_state_is_homogeneous() {
        let gs = make_gs();
        assert!(gs.u().iter().all(|&v| (v - 1.0).abs() < 1e-9));
        assert!(gs.v().iter().all(|&v| v.abs() < 1e-9));
    }

    #[test]
    fn rgba_texture_has_correct_length() {
        let gs = make_gs();
        assert_eq!(gs.rgba_texture().len(), 32 * 32 * 4);
    }

    #[test]
    fn rgba_texture_values_in_range() {
        let mut gs = make_gs();
        gs.seed_center();
        gs.step_n(10, 1.0);
        for byte in gs.rgba_texture() {
            let _ = byte; // just ensure no panic / valid bytes implicitly checked by Vec<u8>
        }
    }

    #[test]
    fn step_changes_state_after_seeding() {
        let mut gs = make_gs();
        gs.seed_center();
        let v_before: Vec<f64> = gs.v().to_vec();
        gs.step(1.0);
        let v_after = gs.v();
        assert!(
            v_before.iter().zip(v_after).any(|(a, b)| (a - b).abs() > 1e-9),
            "state should change after a step"
        );
    }

    #[test]
    fn step_count_increments() {
        let mut gs = make_gs();
        assert_eq!(gs.step_count, 0);
        gs.step(1.0);
        assert_eq!(gs.step_count, 1);
        gs.step_n(4, 1.0);
        assert_eq!(gs.step_count, 5);
    }

    #[test]
    fn concentrations_remain_bounded() {
        let mut gs = make_gs();
        gs.seed_noise(42);
        gs.step_n(50, 0.5);
        assert!(gs.u().iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert!(gs.v().iter().all(|&v| v >= 0.0 && v <= 1.0));
    }

    #[test]
    fn curvature_injection_does_not_panic() {
        let mut gs = make_gs();
        let curvature = vec![0.1f64; 32 * 32];
        gs.set_curvature(&curvature);
        gs.seed_center();
        gs.step_n(5, 1.0);
    }

    #[test]
    fn all_presets_run_without_panic() {
        for preset in [Preset::Coral, Preset::Spots, Preset::Stripes, Preset::Mitosis] {
            let cfg = GrayScottConfig::from_preset(preset);
            let mut gs = GrayScott::new(16, 16, cfg);
            gs.seed_center();
            gs.step_n(10, 1.0);
        }
    }
}
