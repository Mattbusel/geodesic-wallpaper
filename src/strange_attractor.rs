//! Strange attractor renderers.
//!
//! Supports Lorenz, Rossler, Clifford, DeJong, Duffing, and Aizawa attractors,
//! with RK4 integration for continuous-time systems and direct map iteration
//! for discrete-time (iterated function) attractors.

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Which attractor to render and its parameters.
#[derive(Debug, Clone)]
pub enum AttractorType {
    Lorenz {
        sigma: f64,
        rho: f64,
        beta: f64,
    },
    Rossler {
        a: f64,
        b: f64,
        c: f64,
    },
    Clifford {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    },
    DeJong {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
    },
    Duffing {
        alpha: f64,
        beta: f64,
        gamma: f64,
        omega: f64,
        delta: f64,
    },
    Aizawa {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
}

/// A 3-D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Point3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    fn scale(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

// ---------------------------------------------------------------------------
// ColorScheme
// ---------------------------------------------------------------------------

/// Colour palette used when rendering density maps.
#[derive(Debug, Clone, Copy)]
pub enum ColorScheme {
    Fire,
    Ice,
    Plasma,
    Viridis,
    Monochrome,
}

impl ColorScheme {
    /// Map a normalised value `t ∈ [0, 1]` to an RGB colour.
    pub fn to_color(self, t: f64) -> [u8; 3] {
        let t = t.clamp(0.0, 1.0);
        match self {
            ColorScheme::Fire => {
                // black → red → yellow → white
                let r = (t * 3.0).clamp(0.0, 1.0);
                let g = ((t - 0.333) * 3.0).clamp(0.0, 1.0);
                let b = ((t - 0.666) * 3.0).clamp(0.0, 1.0);
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
            }
            ColorScheme::Ice => {
                // black → blue → cyan → white
                let b = (t * 2.0).clamp(0.0, 1.0);
                let g = ((t - 0.5) * 2.0).clamp(0.0, 1.0);
                let r = ((t - 0.75) * 4.0).clamp(0.0, 1.0);
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
            }
            ColorScheme::Plasma => {
                // purple → magenta → orange → yellow
                let r = (0.5 + 0.5 * (t * std::f64::consts::PI * 2.0).sin()).clamp(0.0, 1.0);
                let g = (t * 0.8).clamp(0.0, 1.0);
                let b = (1.0 - t).clamp(0.0, 1.0);
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
            }
            ColorScheme::Viridis => {
                // Approximation of Matplotlib Viridis.
                let r = (0.267 + t * 0.488).clamp(0.0, 1.0);
                let g = (0.005 + t * 0.873).clamp(0.0, 1.0);
                let b = (0.329 + t * 0.145 - t * t * 0.474).clamp(0.0, 1.0);
                [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
            }
            ColorScheme::Monochrome => {
                let v = (t * 255.0) as u8;
                [v, v, v]
            }
        }
    }
}

// ---------------------------------------------------------------------------
// AttractorRenderer
// ---------------------------------------------------------------------------

pub struct AttractorRenderer;

impl AttractorRenderer {
    /// Compute the derivative / next state for the given attractor at `state`.
    ///
    /// For continuous-time systems (Lorenz, Rossler, Duffing, Aizawa) this
    /// returns dx/dt; for iterated maps (Clifford, DeJong) it returns the
    /// next point directly (dt is ignored for maps).
    fn derivative(state: Point3, attractor: &AttractorType) -> Point3 {
        match attractor {
            AttractorType::Lorenz { sigma, rho, beta } => {
                let dx = sigma * (state.y - state.x);
                let dy = state.x * (rho - state.z) - state.y;
                let dz = state.x * state.y - beta * state.z;
                Point3::new(dx, dy, dz)
            }
            AttractorType::Rossler { a, b, c } => {
                let dx = -state.y - state.z;
                let dy = state.x + a * state.y;
                let dz = b + state.z * (state.x - c);
                Point3::new(dx, dy, dz)
            }
            AttractorType::Duffing {
                alpha,
                beta,
                gamma,
                omega,
                delta,
            } => {
                // Duffing oscillator: ẋ = y, ẏ = -δy + αx - βx³ + γcos(ωt)
                // We embed time in z: ż = ω.
                let dx = state.y;
                let dy = -delta * state.y + alpha * state.x - beta * state.x.powi(3)
                    + gamma * state.z.cos();
                let dz = *omega;
                Point3::new(dx, dy, dz)
            }
            AttractorType::Aizawa { a, b, c, d, e, f } => {
                let dx = (state.z - b) * state.x - d * state.y;
                let dy = d * state.x + (state.z - b) * state.y;
                let dz = c + a * state.z
                    - state.z.powi(3) / 3.0
                    - (state.x * state.x + state.y * state.y) * (1.0 + e * state.z)
                    + f * state.z * state.x.powi(3);
                Point3::new(dx, dy, dz)
            }
            // Iterated maps — handled separately in `step`.
            _ => Point3::new(0.0, 0.0, 0.0),
        }
    }

    /// Advance the attractor by one step using RK4 (ODE) or direct map.
    pub fn step(state: Point3, attractor: &AttractorType, dt: f64) -> Point3 {
        match attractor {
            AttractorType::Clifford { a, b, c, d } => {
                let xn = (a * state.y).sin() + c * (a * state.x).cos();
                let yn = (b * state.x).sin() + d * (b * state.y).cos();
                Point3::new(xn, yn, 0.0)
            }
            AttractorType::DeJong { a, b, c, d } => {
                let xn = (a * state.y).sin() - (b * state.x).cos();
                let yn = (c * state.x).sin() - (d * state.y).cos();
                Point3::new(xn, yn, 0.0)
            }
            // RK4 for ODEs.
            _ => {
                let k1 = Self::derivative(state, attractor);
                let k2 = Self::derivative(state.add(k1.scale(dt / 2.0)), attractor);
                let k3 = Self::derivative(state.add(k2.scale(dt / 2.0)), attractor);
                let k4 = Self::derivative(state.add(k3.scale(dt)), attractor);
                let dx = (k1.x + 2.0 * k2.x + 2.0 * k3.x + k4.x) / 6.0;
                let dy = (k1.y + 2.0 * k2.y + 2.0 * k3.y + k4.y) / 6.0;
                let dz = (k1.z + 2.0 * k2.z + 2.0 * k3.z + k4.z) / 6.0;
                Point3::new(
                    state.x + dx * dt,
                    state.y + dy * dt,
                    state.z + dz * dt,
                )
            }
        }
    }

    /// Generate `n_points` attractor points, discarding `warmup` initial points.
    pub fn generate(
        attractor: &AttractorType,
        n_points: usize,
        dt: f64,
        warmup: usize,
    ) -> Vec<Point3> {
        let mut state = Point3::new(0.1, 0.0, 0.0);
        // Warmup.
        for _ in 0..warmup {
            state = Self::step(state, attractor, dt);
            // Guard against divergence.
            if state.x.is_nan() || state.y.is_nan() || state.z.is_nan() {
                state = Point3::new(0.1, 0.0, 0.0);
            }
        }
        let mut points = Vec::with_capacity(n_points);
        for _ in 0..n_points {
            state = Self::step(state, attractor, dt);
            if !state.x.is_nan() && !state.y.is_nan() && !state.z.is_nan() {
                points.push(state);
            }
        }
        points
    }

    /// Project points onto the XY plane.
    pub fn project_xy(points: &[Point3]) -> Vec<(f64, f64)> {
        points.iter().map(|p| (p.x, p.y)).collect()
    }

    /// Project points onto the XZ plane.
    pub fn project_xz(points: &[Point3]) -> Vec<(f64, f64)> {
        points.iter().map(|p| (p.x, p.z)).collect()
    }

    /// Render a density map with logarithmic colour scaling.
    pub fn render_density(
        points: &[(f64, f64)],
        width: u32,
        height: u32,
        color_scheme: ColorScheme,
    ) -> Vec<Vec<[u8; 3]>> {
        if points.is_empty() || width == 0 || height == 0 {
            return vec![vec![[0u8; 3]; width as usize]; height as usize];
        }

        // Compute bounding box.
        let (min_x, max_x) = points.iter().fold((f64::MAX, f64::MIN), |(mn, mx), (x, _)| {
            (mn.min(*x), mx.max(*x))
        });
        let (min_y, max_y) = points.iter().fold((f64::MAX, f64::MIN), |(mn, mx), (_, y)| {
            (mn.min(*y), mx.max(*y))
        });

        let range_x = (max_x - min_x).max(1e-10);
        let range_y = (max_y - min_y).max(1e-10);

        let mut density = vec![vec![0u32; width as usize]; height as usize];

        for (px, py) in points {
            let col = ((px - min_x) / range_x * (width - 1) as f64) as usize;
            let row = ((py - min_y) / range_y * (height - 1) as f64) as usize;
            let col = col.min(width as usize - 1);
            let row = row.min(height as usize - 1);
            density[row][col] = density[row][col].saturating_add(1);
        }

        let max_density = density.iter().flat_map(|r| r.iter()).copied().max().unwrap_or(1);
        let log_max = (max_density as f64 + 1.0).ln();

        density
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&d| {
                        let t = if d == 0 {
                            0.0
                        } else {
                            (d as f64 + 1.0).ln() / log_max
                        };
                        color_scheme.to_color(t)
                    })
                    .collect()
            })
            .collect()
    }

    /// Render an ASCII phase portrait.
    pub fn to_ascii(points: &[(f64, f64)], width: u32, height: u32) -> String {
        if points.is_empty() || width == 0 || height == 0 {
            return String::new();
        }

        let (min_x, max_x) = points.iter().fold((f64::MAX, f64::MIN), |(mn, mx), (x, _)| {
            (mn.min(*x), mx.max(*x))
        });
        let (min_y, max_y) = points.iter().fold((f64::MAX, f64::MIN), |(mn, mx), (_, y)| {
            (mn.min(*y), mx.max(*y))
        });

        let range_x = (max_x - min_x).max(1e-10);
        let range_y = (max_y - min_y).max(1e-10);

        let mut grid = vec![vec![' '; width as usize]; height as usize];

        for (px, py) in points {
            let col = ((px - min_x) / range_x * (width - 1) as f64) as usize;
            let row = ((py - min_y) / range_y * (height - 1) as f64) as usize;
            let col = col.min(width as usize - 1);
            let row = row.min(height as usize - 1);
            grid[row][col] = '*';
        }

        grid.iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lorenz_generates_points() {
        let attractor = AttractorType::Lorenz {
            sigma: 10.0,
            rho: 28.0,
            beta: 8.0 / 3.0,
        };
        let pts = AttractorRenderer::generate(&attractor, 100, 0.01, 50);
        assert_eq!(pts.len(), 100);
    }

    #[test]
    fn clifford_map_generates_points() {
        let attractor = AttractorType::Clifford {
            a: -1.4,
            b: 1.6,
            c: 1.0,
            d: 0.7,
        };
        let pts = AttractorRenderer::generate(&attractor, 200, 0.01, 0);
        assert!(!pts.is_empty());
    }

    #[test]
    fn density_map_dimensions() {
        let pts: Vec<(f64, f64)> = (0..100).map(|i| (i as f64, (i as f64).sin())).collect();
        let grid = AttractorRenderer::render_density(&pts, 16, 16, ColorScheme::Fire);
        assert_eq!(grid.len(), 16);
        assert_eq!(grid[0].len(), 16);
    }

    #[test]
    fn ascii_portrait_non_empty() {
        let pts: Vec<(f64, f64)> = (0..50).map(|i| (i as f64, i as f64)).collect();
        let s = AttractorRenderer::to_ascii(&pts, 20, 10);
        assert!(!s.is_empty());
        assert!(s.contains('*'));
    }

    #[test]
    fn color_scheme_fire_range() {
        for i in 0..=10 {
            let c = ColorScheme::Fire.to_color(i as f64 / 10.0);
            // All channels should be valid u8.
            let _ = c;
        }
    }

    #[test]
    fn project_xy_xz() {
        let pts = vec![Point3::new(1.0, 2.0, 3.0)];
        let xy = AttractorRenderer::project_xy(&pts);
        let xz = AttractorRenderer::project_xz(&pts);
        assert_eq!(xy[0], (1.0, 2.0));
        assert_eq!(xz[0], (1.0, 3.0));
    }
}
