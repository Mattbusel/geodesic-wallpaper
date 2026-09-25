//! Extended Gray-Scott reaction-diffusion — 3-chemical Turing pattern system
//! plus the standard 2-chemical Gray-Scott model.
//!
//! The 3-chemical system follows a Gierer-Meinhardt-like scheme:
//!   du/dt = Du·∇²u − u·v² + a·(1 − u)
//!   dv/dt = Dv·∇²v + u·v² − (b + a)·v
//!   dw/dt = Dw·∇²w + v  − c·w

// ---------------------------------------------------------------------------
// LCG (for seeding)
// ---------------------------------------------------------------------------

const LCG_A: u64 = 6_364_136_223_846_793_005;
const LCG_C: u64 = 1_442_695_040_888_963_407;

fn lcg_next(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(LCG_A).wrapping_add(LCG_C);
    (*state >> 33) as f64 / 0x7FFF_FFFFu64 as f64 // [0, 1)
}

// ---------------------------------------------------------------------------
// 5-point Laplacian with wrap-around boundary conditions
// ---------------------------------------------------------------------------

/// Compute the discrete Laplacian at (x, y) using a 5-point stencil.
pub fn laplacian(grid: &[f64], width: usize, height: usize, x: usize, y: usize) -> f64 {
    let w = width;
    let h = height;
    let idx = |xi: usize, yi: usize| yi * w + xi;

    let xp = (x + 1) % w;
    let xm = (x + w - 1) % w;
    let yp = (y + 1) % h;
    let ym = (y + h - 1) % h;

    grid[idx(xp, y)] + grid[idx(xm, y)] + grid[idx(x, yp)] + grid[idx(x, ym)]
        - 4.0 * grid[idx(x, y)]
}

// ---------------------------------------------------------------------------
// 3-Chemical Turing system
// ---------------------------------------------------------------------------

/// Diffusion and reaction-rate parameters for the 3-chemical system.
#[derive(Debug, Clone)]
pub struct TuringParams {
    pub du: f64,
    pub dv: f64,
    pub dw: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub dt: f64,
}

/// Grid holding concentrations of three chemicals u, v, w.
pub struct ThreeChemGrid {
    pub width: usize,
    pub height: usize,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
    pub w: Vec<f64>,
}

impl ThreeChemGrid {
    /// Initialise with u=1, v=0, w=0 everywhere and random perturbations in
    /// the central 20% of the grid.
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        let mut u = vec![1.0f64; n];
        let mut v = vec![0.0f64; n];
        let mut w = vec![0.0f64; n];

        let mut rng: u64 = 0xDEAD_BEEF_1234_5678;
        let cx = width / 2;
        let cy = height / 2;
        let rx = (width / 5).max(1);
        let ry = (height / 5).max(1);

        for yi in 0..height {
            for xi in 0..width {
                let in_center = (xi as i64 - cx as i64).abs() < rx as i64
                    && (yi as i64 - cy as i64).abs() < ry as i64;
                if in_center {
                    let idx = yi * width + xi;
                    u[idx] = 0.5 + 0.1 * lcg_next(&mut rng);
                    v[idx] = 0.25 + 0.1 * lcg_next(&mut rng);
                    w[idx] = 0.1 * lcg_next(&mut rng);
                }
            }
        }

        Self {
            width,
            height,
            u,
            v,
            w,
        }
    }

    /// Euler step for all three chemicals.
    pub fn step(&mut self, p: &TuringParams) {
        let n = self.width * self.height;
        let mut du = vec![0.0f64; n];
        let mut dv = vec![0.0f64; n];
        let mut dw = vec![0.0f64; n];

        for yi in 0..self.height {
            for xi in 0..self.width {
                let idx = yi * self.width + xi;
                let u = self.u[idx];
                let v = self.v[idx];
                let w = self.w[idx];

                let lap_u = laplacian(&self.u, self.width, self.height, xi, yi);
                let lap_v = laplacian(&self.v, self.width, self.height, xi, yi);
                let lap_w = laplacian(&self.w, self.width, self.height, xi, yi);

                let uvv = u * v * v;
                du[idx] = p.du * lap_u - uvv + p.a * (1.0 - u);
                dv[idx] = p.dv * lap_v + uvv - (p.b + p.a) * v;
                dw[idx] = p.dw * lap_w + v - p.c * w;
            }
        }

        for i in 0..n {
            self.u[i] = (self.u[i] + p.dt * du[i]).clamp(0.0, 1.0);
            self.v[i] = (self.v[i] + p.dt * dv[i]).clamp(0.0, 1.0);
            self.w[i] = (self.w[i] + p.dt * dw[i]).clamp(0.0, 1.0);
        }
    }

    /// Run `steps` Euler iterations.
    pub fn run(&mut self, params: &TuringParams, steps: usize) {
        for _ in 0..steps {
            self.step(params);
        }
    }

    /// Map u→R, v→G, w→B scaled to [0, 255] and return an RGB byte buffer.
    pub fn to_rgb_image(&self, width: u32, height: u32) -> Vec<u8> {
        let src_w = self.width;
        let src_h = self.height;
        let dst_w = width as usize;
        let dst_h = height as usize;
        let mut out = vec![0u8; dst_w * dst_h * 3];

        for dy in 0..dst_h {
            for dx in 0..dst_w {
                // Nearest-neighbour sampling from source grid.
                let sx = (dx * src_w / dst_w.max(1)).min(src_w.saturating_sub(1));
                let sy = (dy * src_h / dst_h.max(1)).min(src_h.saturating_sub(1));
                let src_idx = sy * src_w + sx;
                let dst_idx = (dy * dst_w + dx) * 3;

                out[dst_idx] = (self.u[src_idx].clamp(0.0, 1.0) * 255.0).round() as u8;
                out[dst_idx + 1] = (self.v[src_idx].clamp(0.0, 1.0) * 255.0).round() as u8;
                out[dst_idx + 2] = (self.w[src_idx].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Standard 2-chemical Gray-Scott model
// ---------------------------------------------------------------------------

/// Gray-Scott model parameters.
#[derive(Debug, Clone)]
pub struct GrayScottParams {
    /// Feed rate.
    pub f: f64,
    /// Kill rate.
    pub k: f64,
    pub du: f64,
    pub dv: f64,
    pub dt: f64,
}

/// Grid holding concentrations of two chemicals u and v.
pub struct TwoChemGrid {
    pub width: usize,
    pub height: usize,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
}

impl TwoChemGrid {
    /// Initialise u=1, v=0 with a small random seeded region in the centre.
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        let mut u = vec![1.0f64; n];
        let mut v = vec![0.0f64; n];

        let mut rng: u64 = 0xCAFE_BABE_0000_0001;
        let cx = width / 2;
        let cy = height / 2;
        let r = (width.min(height) / 10).max(1);

        for yi in 0..height {
            for xi in 0..width {
                if (xi as i64 - cx as i64).abs() < r as i64
                    && (yi as i64 - cy as i64).abs() < r as i64
                {
                    let idx = yi * width + xi;
                    u[idx] = 0.5 + 0.01 * lcg_next(&mut rng);
                    v[idx] = 0.25 + 0.01 * lcg_next(&mut rng);
                }
            }
        }

        Self {
            width,
            height,
            u,
            v,
        }
    }

    /// Single Euler step of the Gray-Scott equations.
    pub fn step(&mut self, p: &GrayScottParams) {
        let n = self.width * self.height;
        let mut du = vec![0.0f64; n];
        let mut dv = vec![0.0f64; n];

        for yi in 0..self.height {
            for xi in 0..self.width {
                let idx = yi * self.width + xi;
                let u = self.u[idx];
                let v = self.v[idx];
                let uvv = u * v * v;

                let lap_u = laplacian(&self.u, self.width, self.height, xi, yi);
                let lap_v = laplacian(&self.v, self.width, self.height, xi, yi);

                du[idx] = p.du * lap_u - uvv + p.f * (1.0 - u);
                dv[idx] = p.dv * lap_v + uvv - (p.f + p.k) * v;
            }
        }

        for i in 0..n {
            self.u[i] = (self.u[i] + p.dt * du[i]).clamp(0.0, 1.0);
            self.v[i] = (self.v[i] + p.dt * dv[i]).clamp(0.0, 1.0);
        }
    }

    /// Return the v-channel mapped to [0, 255] grayscale.
    pub fn to_grayscale(&self) -> Vec<u8> {
        self.v
            .iter()
            .map(|&v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_turing_params() -> TuringParams {
        TuringParams {
            du: 0.16,
            dv: 0.08,
            dw: 0.04,
            a: 0.06,
            b: 0.06,
            c: 0.1,
            dt: 1.0,
        }
    }

    fn default_gs_params() -> GrayScottParams {
        GrayScottParams {
            f: 0.055,
            k: 0.062,
            du: 0.2097,
            dv: 0.105,
            dt: 1.0,
        }
    }

    #[test]
    fn three_chem_grid_initializes_correct_size() {
        let g = ThreeChemGrid::new(32, 32);
        assert_eq!(g.u.len(), 32 * 32);
        assert_eq!(g.v.len(), 32 * 32);
        assert_eq!(g.w.len(), 32 * 32);
    }

    #[test]
    fn three_chem_step_does_not_nan() {
        let mut g = ThreeChemGrid::new(16, 16);
        let p = default_turing_params();
        g.step(&p);
        assert!(g.u.iter().all(|v| v.is_finite()), "u contains NaN/Inf");
        assert!(g.v.iter().all(|v| v.is_finite()), "v contains NaN/Inf");
        assert!(g.w.iter().all(|v| v.is_finite()), "w contains NaN/Inf");
    }

    #[test]
    fn to_rgb_returns_correct_size() {
        let g = ThreeChemGrid::new(32, 32);
        let img = g.to_rgb_image(64, 48);
        assert_eq!(img.len(), 64 * 48 * 3);
    }

    #[test]
    fn two_chem_grid_initializes_correct_size() {
        let g = TwoChemGrid::new(20, 20);
        assert_eq!(g.u.len(), 20 * 20);
        assert_eq!(g.v.len(), 20 * 20);
    }

    #[test]
    fn two_chem_step_does_not_nan() {
        let mut g = TwoChemGrid::new(16, 16);
        let p = default_gs_params();
        g.step(&p);
        assert!(g.u.iter().all(|v| v.is_finite()));
        assert!(g.v.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn grayscale_correct_length() {
        let g = TwoChemGrid::new(10, 10);
        assert_eq!(g.to_grayscale().len(), 100);
    }

    #[test]
    fn u_approximately_conserved_early() {
        // After one step with low feed/kill rates, average u should stay close to 1.
        let mut g = TwoChemGrid::new(16, 16);
        let p = default_gs_params();
        let before: f64 = g.u.iter().sum::<f64>() / g.u.len() as f64;
        g.step(&p);
        let after: f64 = g.u.iter().sum::<f64>() / g.u.len() as f64;
        // Allow up to 10% drift in one step.
        assert!(
            (before - after).abs() < 0.1,
            "u avg changed from {} to {}",
            before,
            after
        );
    }
}
