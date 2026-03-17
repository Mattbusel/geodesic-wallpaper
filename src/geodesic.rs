use crate::surface::Surface;

/// State of a single geodesic on the surface
#[derive(Clone)]
pub struct Geodesic {
    /// Current parameter coordinates (u, v)
    pub u: f32,
    pub v: f32,
    /// Current velocity (du/dt, dv/dt)
    pub du: f32,
    pub dv: f32,
    /// Age in frames
    pub age: usize,
    /// Max lifetime
    pub max_age: usize,
    /// Color index
    pub color_idx: usize,
    /// Whether active
    pub alive: bool,
}

impl Geodesic {
    pub fn new(u: f32, v: f32, du: f32, dv: f32, max_age: usize, color_idx: usize) -> Self {
        Self { u, v, du, dv, age: 0, max_age, color_idx, alive: true }
    }

    /// Advance one step using RK4 on the geodesic equation.
    /// d²x^k/dt² + Γ^k_ij (dx^i/dt)(dx^j/dt) = 0
    pub fn step(&mut self, surface: &dyn Surface, dt: f32) {
        let (u, v, du, dv) = (self.u, self.v, self.du, self.dv);

        let deriv = |u: f32, v: f32, du: f32, dv: f32| -> (f32, f32, f32, f32) {
            let (u_w, v_w) = surface.wrap(u, v);
            let g = surface.christoffel(u_w, v_w);
            // Acceleration from geodesic equation
            let acc_u = -(g[0][0][0] * du * du
                        + 2.0 * g[0][0][1] * du * dv
                        + g[0][1][1] * dv * dv);
            let acc_v = -(g[1][0][0] * du * du
                        + 2.0 * g[1][0][1] * du * dv
                        + g[1][1][1] * dv * dv);
            (du, dv, acc_u, acc_v)
        };

        // RK4
        let (k1u, k1v, k1du, k1dv) = deriv(u, v, du, dv);
        let (k2u, k2v, k2du, k2dv) = deriv(
            u + 0.5 * dt * k1u, v + 0.5 * dt * k1v,
            du + 0.5 * dt * k1du, dv + 0.5 * dt * k1dv,
        );
        let (k3u, k3v, k3du, k3dv) = deriv(
            u + 0.5 * dt * k2u, v + 0.5 * dt * k2v,
            du + 0.5 * dt * k2du, dv + 0.5 * dt * k2dv,
        );
        let (k4u, k4v, k4du, k4dv) = deriv(
            u + dt * k3u, v + dt * k3v,
            du + dt * k3du, dv + dt * k3dv,
        );

        self.u += dt / 6.0 * (k1u + 2.0 * k2u + 2.0 * k3u + k4u);
        self.v += dt / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        self.du += dt / 6.0 * (k1du + 2.0 * k2du + 2.0 * k3du + k4du);
        self.dv += dt / 6.0 * (k1dv + 2.0 * k2dv + 2.0 * k3dv + k4dv);

        let (u_w, v_w) = surface.wrap(self.u, self.v);
        self.u = u_w;
        self.v = v_w;

        // Renormalize velocity to unit metric speed after each RK4 step.
        // Without this, floating-point error accumulates over hundreds of
        // frames: on a sphere the geodesic constraint g_ij du^i du^j = const
        // is not preserved by the integrator alone, so trails shrink or
        // stretch unnaturally over a ~300-frame lifetime.
        let g = surface.metric(self.u, self.v);
        let speed_sq = g[0][0] * self.du * self.du
            + 2.0 * g[0][1] * self.du * self.dv
            + g[1][1] * self.dv * self.dv;
        if speed_sq > 1e-12 {
            let inv_speed = 1.0 / speed_sq.sqrt();
            self.du *= inv_speed;
            self.dv *= inv_speed;
        }

        self.age += 1;

        if self.age >= self.max_age {
            self.alive = false;
        }
    }
}
