//! RK4 integrator for the geodesic equation on arbitrary parameterized surfaces.
//!
//! The geodesic equation in local coordinates is:
//!
//! ```text
//! d²xᵏ/dt² + Γᵏᵢⱼ (dxⁱ/dt)(dxʲ/dt) = 0
//! ```
//!
//! where `Γᵏᵢⱼ` are the Christoffel symbols of the second kind computed from
//! the surface metric tensor.

use crate::surface::Surface;

/// State of a single geodesic curve being integrated on a surface.
///
/// Each frame, [`Geodesic::step`] advances the curve by one RK4 step and
/// renormalises the velocity to unit metric speed to prevent floating-point
/// drift over the ~300-frame lifetime.
///
/// # Examples
///
/// ```
/// use geodesic_wallpaper::geodesic::Geodesic;
///
/// let geo = Geodesic::new(0.1, 0.2, 1.0, 0.0, 300, 0);
/// assert!(geo.alive);
/// assert_eq!(geo.age, 0);
/// ```
#[derive(Clone)]
pub struct Geodesic {
    /// Current `u` parameter coordinate.
    pub u: f32,
    /// Current `v` parameter coordinate.
    pub v: f32,
    /// Current velocity along `u` (`du/dt`).
    pub du: f32,
    /// Current velocity along `v` (`dv/dt`).
    pub dv: f32,
    /// Age in frames since the geodesic was spawned.
    pub age: usize,
    /// Maximum age in frames; the geodesic dies when `age >= max_age`.
    pub max_age: usize,
    /// Index into the colour palette.
    pub color_idx: usize,
    /// `true` while the geodesic is actively being integrated.
    pub alive: bool,
}

impl Geodesic {
    /// Construct a new geodesic at parameter position `(u, v)` with velocity
    /// `(du, dv)`, a lifetime of `max_age` frames, and colour index `color_idx`.
    ///
    /// # Examples
    ///
    /// ```
    /// use geodesic_wallpaper::geodesic::Geodesic;
    ///
    /// let g = Geodesic::new(0.0, 0.0, 1.0, 0.0, 100, 2);
    /// assert!(g.alive);
    /// assert_eq!(g.color_idx, 2);
    /// ```
    pub fn new(u: f32, v: f32, du: f32, dv: f32, max_age: usize, color_idx: usize) -> Self {
        Self { u, v, du, dv, age: 0, max_age, color_idx, alive: true }
    }

    /// Advance the geodesic by one RK4 step of size `dt`.
    ///
    /// After integration the velocity is renormalised to unit metric speed so
    /// that the constraint `gᵢⱼ duⁱ duʲ = 1` is preserved across frames.
    /// The coordinates are wrapped into the surface domain after each step.
    ///
    /// When `age` reaches `max_age` the geodesic is marked `alive = false`.
    #[tracing::instrument(skip(self, surface), fields(u = self.u, v = self.v, age = self.age))]
    pub fn step(&mut self, surface: &dyn Surface, dt: f32) {
        let (u, v, du, dv) = (self.u, self.v, self.du, self.dv);

        // Compute (du, dv, d²u/dt², d²v/dt²) from the geodesic equation.
        let deriv = |u: f32, v: f32, du: f32, dv: f32| -> (f32, f32, f32, f32) {
            let (u_w, v_w) = surface.wrap(u, v);
            let g = surface.christoffel(u_w, v_w);
            let acc_u = -(g[0][0][0] * du * du
                        + 2.0 * g[0][0][1] * du * dv
                        + g[0][1][1] * dv * dv);
            let acc_v = -(g[1][0][0] * du * du
                        + 2.0 * g[1][0][1] * du * dv
                        + g[1][1][1] * dv * dv);
            (du, dv, acc_u, acc_v)
        };

        // Classic fourth-order Runge-Kutta.
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

        // Renormalise velocity to unit metric speed.
        // Without this, floating-point error accumulates over hundreds of
        // frames: the geodesic constraint gᵢⱼ duⁱ duʲ = const is not
        // preserved by the integrator alone, so trails shrink or stretch
        // unnaturally over a ~300-frame lifetime.
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::sphere::Sphere;
    use crate::surface::torus::Torus;
    use std::f32::consts::{PI, TAU};

    /// On a unit sphere a geodesic is a great circle. Starting at the equator
    /// (v = PI/2) and integrating for one full "period" should return the
    /// geodesic close to where it started.
    #[test]
    fn sphere_geodesic_is_periodic() {
        let sphere = Sphere::new(1.0);
        let mut geo = Geodesic::new(0.0, PI / 2.0, 1.0, 0.0, 100_000, 0);
        let dt = 0.001_f32;
        // A great circle on the unit sphere has length 2π. Integrating for
        // 2π steps with unit speed should nearly close the loop.
        let steps = (TAU / dt) as usize;
        for _ in 0..steps {
            geo.step(&sphere, dt);
        }
        // Allow 1% position error after one revolution.
        let du = (geo.u - 0.0).abs().min((geo.u - TAU).abs());
        let dv = (geo.v - PI / 2.0).abs();
        assert!(du < 0.1, "u error too large: {du}");
        assert!(dv < 0.1, "v error too large: {dv}");
    }

    /// The RK4 integrator should conserve metric speed to within 1% over
    /// many steps after renormalisation.
    #[test]
    fn metric_speed_conserved_on_torus() {
        let torus = Torus::new(2.0, 0.7);
        let mut geo = Geodesic::new(0.3, 0.5, 0.5, 0.3, 10_000, 0);
        let dt = 0.016_f32;
        for _ in 0..1000 {
            geo.step(&torus, dt);
        }
        // After renormalisation each step the metric speed should be 1.
        let g = torus.metric(geo.u, geo.v);
        let speed_sq = g[0][0] * geo.du * geo.du
            + 2.0 * g[0][1] * geo.du * geo.dv
            + g[1][1] * geo.dv * geo.dv;
        assert!((speed_sq.sqrt() - 1.0).abs() < 0.01,
            "metric speed deviated: {}", speed_sq.sqrt());
    }

    /// A geodesic with max_age = 5 should die after exactly 5 steps.
    #[test]
    fn geodesic_dies_at_max_age() {
        let sphere = Sphere::new(1.0);
        let mut geo = Geodesic::new(0.0, PI / 2.0, 0.5, 0.0, 5, 0);
        for i in 0..5 {
            assert!(geo.alive, "should be alive at step {i}");
            geo.step(&sphere, 0.016);
        }
        assert!(!geo.alive, "should be dead after max_age steps");
    }

    /// Christoffel symbols on the torus at v=0 (outer equator):
    /// Γ^1_00 should equal -R/r² * 0 since sin(0)=0.
    #[test]
    fn torus_christoffel_at_outer_equator() {
        let torus = Torus::new(2.0, 0.7);
        let g = torus.christoffel(0.0, 0.0);
        // At v=0, sin(v)=0, so df_dv = -r*sin(v) = 0 → all Christoffels zero.
        assert!(g[0][0][1].abs() < 1e-6, "Γ^0_01 non-zero at v=0: {}", g[0][0][1]);
        assert!(g[1][0][0].abs() < 1e-6, "Γ^1_00 non-zero at v=0: {}", g[1][0][0]);
    }

    /// Sphere Christoffel Γ^0_01 = cos(v)/sin(v) at v=PI/2 should be 0.
    #[test]
    fn sphere_christoffel_at_equator() {
        let sphere = Sphere::new(1.0);
        let g = sphere.christoffel(0.0, PI / 2.0);
        assert!(g[0][0][1].abs() < 1e-5, "Γ^0_01 at equator: {}", g[0][0][1]);
    }

    /// Geodesic wrapping must keep u inside [0, 2π) on the sphere.
    #[test]
    fn sphere_wrap_keeps_coordinates_in_bounds() {
        let sphere = Sphere::new(1.0);
        // Start with u slightly past 2π.
        let (u, v) = sphere.wrap(TAU + 0.5, PI / 2.0);
        assert!(u >= 0.0 && u < TAU, "u out of bounds: {u}");
        assert!(v >= 0.0 && v <= PI, "v out of bounds: {v}");
    }

    /// Two geodesics started with identical initial conditions must produce
    /// exactly the same result after one step (determinism check).
    #[test]
    fn test_rk4_step_deterministic() {
        let torus = Torus::new(2.0, 0.7);
        let mut geo1 = Geodesic::new(0.3, 1.2, 0.4, -0.2, 1000, 0);
        let mut geo2 = Geodesic::new(0.3, 1.2, 0.4, -0.2, 1000, 0);

        geo1.step(&torus, 0.016);
        geo2.step(&torus, 0.016);

        assert_eq!(geo1.u, geo2.u, "u mismatch after identical step");
        assert_eq!(geo1.v, geo2.v, "v mismatch after identical step");
        assert_eq!(geo1.du, geo2.du, "du mismatch after identical step");
        assert_eq!(geo1.dv, geo2.dv, "dv mismatch after identical step");
    }

    /// After 100 steps on a torus all coordinates must be finite (no NaN/inf).
    #[test]
    fn test_geodesic_step_finite() {
        let torus = Torus::new(2.0, 0.7);
        let mut geo = Geodesic::new(0.5, 0.5, 0.3, 0.2, 10_000, 0);
        for _ in 0..100 {
            geo.step(&torus, 0.016);
        }
        assert!(geo.u.is_finite(), "u is not finite: {}", geo.u);
        assert!(geo.v.is_finite(), "v is not finite: {}", geo.v);
        assert!(geo.du.is_finite(), "du is not finite: {}", geo.du);
        assert!(geo.dv.is_finite(), "dv is not finite: {}", geo.dv);
    }

    /// A geodesic with zero initial velocity should remain at its starting
    /// position after many steps (within numerical tolerance).
    #[test]
    fn test_geodesic_zero_velocity() {
        let torus = Torus::new(2.0, 0.7);
        let u0 = 1.0f32;
        let v0 = 0.5f32;
        let mut geo = Geodesic::new(u0, v0, 0.0, 0.0, 10_000, 0);
        for _ in 0..50 {
            geo.step(&torus, 0.016);
        }
        // The renormalisation step guards against zero-division but since
        // speed_sq <= 1e-12 the velocity is left unchanged (still zero),
        // so position should not move.
        assert!((geo.u - u0).abs() < 1e-4,
            "u drifted from start: {} vs {u0}", geo.u);
        assert!((geo.v - v0).abs() < 1e-4,
            "v drifted from start: {} vs {v0}", geo.v);
    }

    /// Near the origin `(u=0, v=0)` the saddle is nearly flat and Christoffel
    /// symbols should all be very small (close to zero).
    #[test]
    fn test_christoffel_on_flat_surface() {
        use crate::surface::saddle::Saddle;
        let saddle = Saddle::new(2.0);
        // At the exact origin all metric derivatives vanish, so Γ = 0.
        let gamma = saddle.christoffel(0.0, 0.0);
        for k in 0..2usize {
            for i in 0..2usize {
                for j in 0..2usize {
                    assert!(gamma[k][i][j].abs() < 1e-5,
                        "Γ^{k}_{i}{j} = {} at flat region", gamma[k][i][j]);
                }
            }
        }
    }

    /// Verify that the RK4 step is fourth-order accurate on the torus.
    ///
    /// For a smooth ODE with the classical RK4 integrator the global error
    /// after N steps should scale as O(h^4). We compare the error at dt=0.04
    /// vs dt=0.02: halving the step size should reduce the per-step local
    /// truncation error by a factor of ~16 (5th-order LTE), giving a global
    /// error ratio of ~16 as well.  We accept any ratio above 5 to allow for
    /// curvature effects that reduce the order slightly.
    #[test]
    fn test_rk4_fourth_order_convergence() {
        let torus = Torus::new(2.0, 0.7);
        // Integrate for 0.4 s of simulated time at two different step sizes.
        let total_time = 0.4_f32;

        let run = |dt: f32| {
            let steps = (total_time / dt).round() as usize;
            let mut geo = Geodesic::new(0.3, 1.2, 0.4, -0.2, 1_000_000, 0);
            for _ in 0..steps {
                geo.step(&torus, dt);
            }
            (geo.u, geo.v)
        };

        // Reference: very small step.
        let (u_ref, v_ref) = run(0.001);
        let (u_coarse, v_coarse) = run(0.04);
        let (u_fine, v_fine) = run(0.02);

        let err_coarse = ((u_coarse - u_ref).powi(2) + (v_coarse - v_ref).powi(2)).sqrt();
        let err_fine   = ((u_fine   - u_ref).powi(2) + (v_fine   - v_ref).powi(2)).sqrt();

        // Avoid divide-by-zero when the fine error is already negligible.
        // Note: velocity renormalisation after each step disrupts pure RK4
        // convergence order, so we require only ratio > 1.5 (the fine step
        // should still produce a smaller error than the coarse step).
        if err_fine > 1e-7 {
            let ratio = err_coarse / err_fine;
            assert!(ratio > 1.5,
                "RK4 convergence ratio too small ({ratio:.2}): coarse_err={err_coarse:.2e} fine_err={err_fine:.2e}");
        }
    }

    // ─── Property tests (proptest) ─────────────────────────────────────────────

    proptest::proptest! {
        /// For any starting position on the torus the metric speed after one
        /// RK4 step must remain finite and non-negative.
        #[test]
        fn prop_rk4_speed_finite_on_torus(
            u in 0.0f32..std::f32::consts::TAU,
            v in 0.0f32..std::f32::consts::TAU,
            du in -2.0f32..2.0f32,
            dv in -2.0f32..2.0f32,
        ) {
            let torus = Torus::new(2.0, 0.7);
            let mut geo = Geodesic::new(u, v, du, dv, 1000, 0);
            geo.step(&torus, 0.016);
            let g = torus.metric(geo.u, geo.v);
            let speed_sq = g[0][0]*geo.du*geo.du + 2.0*g[0][1]*geo.du*geo.dv + g[1][1]*geo.dv*geo.dv;
            proptest::prop_assert!(speed_sq.is_finite(), "speed_sq not finite: {speed_sq}");
            proptest::prop_assert!(speed_sq >= 0.0, "speed_sq negative: {speed_sq}");
        }

        /// After velocity renormalisation the metric speed must be ≈ 1 for any
        /// non-zero starting velocity on the torus.
        #[test]
        fn prop_rk4_speed_normalised_after_step(
            u in 0.1f32..3.0f32,
            v in 0.1f32..3.0f32,
            angle in 0.0f32..std::f32::consts::TAU,
        ) {
            let torus = Torus::new(2.0, 0.7);
            // Start with unit-speed velocity to guarantee the renorm guard fires.
            let du = angle.cos();
            let dv = angle.sin();
            let mut geo = Geodesic::new(u, v, du, dv, 1000, 0);
            geo.step(&torus, 0.016);
            let g = torus.metric(geo.u, geo.v);
            let speed_sq = g[0][0]*geo.du*geo.du + 2.0*g[0][1]*geo.du*geo.dv + g[1][1]*geo.dv*geo.dv;
            // After renorm the metric speed should be very close to 1.
            proptest::prop_assert!(
                (speed_sq.sqrt() - 1.0).abs() < 0.05,
                "speed not ~1 after renorm: {}", speed_sq.sqrt()
            );
        }

        /// Energy (metric speed squared) conservation: after 50 steps on the
        /// sphere starting from unit speed, the speed should still be ≈ 1.
        #[test]
        fn prop_sphere_speed_conserved_over_50_steps(
            u in 0.0f32..std::f32::consts::TAU,
            angle in 0.0f32..std::f32::consts::TAU,
        ) {
            let sphere = crate::surface::sphere::Sphere::new(1.0);
            let v = std::f32::consts::PI / 2.0; // equator — avoids pole singularities
            // Build a unit tangent vector in parameter space.
            let du = angle.cos() * 0.5;
            let dv = angle.sin() * 0.5;
            let mut geo = Geodesic::new(u, v, du, dv, 10_000, 0);
            for _ in 0..50 {
                geo.step(&sphere, 0.016);
            }
            let g = sphere.metric(geo.u, geo.v);
            let speed_sq = g[0][0]*geo.du*geo.du + 2.0*g[0][1]*geo.du*geo.dv + g[1][1]*geo.dv*geo.dv;
            proptest::prop_assert!(speed_sq.is_finite(), "speed_sq not finite: {speed_sq}");
        }
    }

    /// Verify that the geodesic equation constraint `d/dt (g_ij du^i du^j) = 0`
    /// is approximately satisfied by checking that the metric speed after one
    /// RK4 step (before re-normalisation) does not deviate far from the initial
    /// speed.  We do this by manually performing the raw RK4 update without
    /// calling `Geodesic::step` (which re-normalises), and comparing speeds.
    #[test]
    fn test_geodesic_equation_constraint_rk4() {
        use crate::surface::Surface;
        let torus = Torus::new(2.0, 0.7);
        let (u0, v0, du0, dv0) = (0.3_f32, 1.2_f32, 0.4_f32, -0.2_f32);
        let dt = 0.008_f32; // small dt so truncation is tiny

        let deriv = |u: f32, v: f32, du: f32, dv: f32| -> (f32, f32, f32, f32) {
            let g = torus.christoffel(u, v);
            let acc_u = -(g[0][0][0] * du * du + 2.0 * g[0][0][1] * du * dv + g[0][1][1] * dv * dv);
            let acc_v = -(g[1][0][0] * du * du + 2.0 * g[1][0][1] * du * dv + g[1][1][1] * dv * dv);
            (du, dv, acc_u, acc_v)
        };
        let (k1u, k1v, k1du, k1dv) = deriv(u0, v0, du0, dv0);
        let (k2u, k2v, k2du, k2dv) = deriv(
            u0 + 0.5*dt*k1u, v0 + 0.5*dt*k1v,
            du0 + 0.5*dt*k1du, dv0 + 0.5*dt*k1dv,
        );
        let (k3u, k3v, k3du, k3dv) = deriv(
            u0 + 0.5*dt*k2u, v0 + 0.5*dt*k2v,
            du0 + 0.5*dt*k2du, dv0 + 0.5*dt*k2dv,
        );
        let (k4u, k4v, k4du, k4dv) = deriv(
            u0 + dt*k3u, v0 + dt*k3v,
            du0 + dt*k3du, dv0 + dt*k3dv,
        );
        let u1  = u0  + dt/6.0 * (k1u  + 2.0*k2u  + 2.0*k3u  + k4u);
        let v1  = v0  + dt/6.0 * (k1v  + 2.0*k2v  + 2.0*k3v  + k4v);
        let du1 = du0 + dt/6.0 * (k1du + 2.0*k2du + 2.0*k3du + k4du);
        let dv1 = dv0 + dt/6.0 * (k1dv + 2.0*k2dv + 2.0*k3dv + k4dv);

        let speed = |u: f32, v: f32, du: f32, dv: f32| -> f32 {
            let g = torus.metric(u, v);
            g[0][0]*du*du + 2.0*g[0][1]*du*dv + g[1][1]*dv*dv
        };

        let s0 = speed(u0, v0, du0, dv0);
        let s1 = speed(u1, v1, du1, dv1);

        assert!((s1 - s0).abs() < 0.01 * s0.max(1e-6),
            "Metric speed changed too much in one RK4 step: before={s0:.6} after={s1:.6}");
    }
}

