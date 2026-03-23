//! Particle system with physics simulation.
//!
//! Provides a 2-D particle emitter / simulator / renderer with Euler
//! integration and a declarative force model.  Zero external dependencies.

// ---------------------------------------------------------------------------
// Vec2
// ---------------------------------------------------------------------------

/// A 2-D floating-point vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Vec2 { x, y }
    }

    pub fn zero() -> Self {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }

    pub fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }

    pub fn scale(self, s: f64) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }

    pub fn magnitude(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(self) -> Vec2 {
        let m = self.magnitude();
        if m < 1e-12 {
            Vec2::zero()
        } else {
            self.scale(1.0 / m)
        }
    }

    pub fn dot(self, rhs: Vec2) -> f64 {
        self.x * rhs.x + self.y * rhs.y
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::add(self, rhs)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::sub(self, rhs)
    }
}

impl std::ops::Mul<f64> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f64) -> Vec2 {
        Vec2::scale(self, rhs)
    }
}

// ---------------------------------------------------------------------------
// Particle
// ---------------------------------------------------------------------------

/// A single particle in the simulation.
#[derive(Debug, Clone)]
pub struct Particle {
    pub id: u64,
    pub position: Vec2,
    pub velocity: Vec2,
    pub mass: f64,
    pub charge: f64,
    /// Total lifetime in milliseconds.
    pub lifetime_ms: f64,
    /// Current age in milliseconds.
    pub age_ms: f64,
    pub color: [u8; 3],
}

impl Particle {
    /// True if this particle has exceeded its lifetime.
    pub fn is_expired(&self) -> bool {
        self.age_ms >= self.lifetime_ms
    }

    /// Fraction of lifetime elapsed, in [0.0, 1.0].
    pub fn life_fraction(&self) -> f64 {
        (self.age_ms / self.lifetime_ms.max(1e-12)).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// Force
// ---------------------------------------------------------------------------

/// A physics force applied to all particles each time step.
#[derive(Debug, Clone)]
pub enum Force {
    /// Downward gravity acceleration.
    Gravity { strength: f64 },
    /// Radial repulsion from particle positions (inter-particle or global).
    Repulsion { radius: f64, strength: f64 },
    /// Attraction towards a fixed world-space point.
    Attraction { target: Vec2, strength: f64 },
    /// Constant wind in a given direction.
    Wind { direction: Vec2, strength: f64 },
}

// ---------------------------------------------------------------------------
// ParticleSystem
// ---------------------------------------------------------------------------

/// Manages a collection of particles, updates physics, and renders to a pixel buffer.
pub struct ParticleSystem {
    particles: Vec<Particle>,
    next_id: u64,
}

impl ParticleSystem {
    pub fn new() -> Self {
        ParticleSystem {
            particles: Vec::new(),
            next_id: 1,
        }
    }

    /// Emit a new particle, returning its unique id.
    pub fn emit(
        &mut self,
        position: Vec2,
        velocity: Vec2,
        mass: f64,
        lifetime_ms: f64,
        color: [u8; 3],
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.particles.push(Particle {
            id,
            position,
            velocity,
            mass: mass.max(1e-12),
            charge: 0.0,
            lifetime_ms: lifetime_ms.max(1.0),
            age_ms: 0.0,
            color,
        });
        id
    }

    /// Emit with an explicit charge value.
    pub fn emit_charged(
        &mut self,
        position: Vec2,
        velocity: Vec2,
        mass: f64,
        charge: f64,
        lifetime_ms: f64,
        color: [u8; 3],
    ) -> u64 {
        let id = self.emit(position, velocity, mass, lifetime_ms, color);
        if let Some(p) = self.particles.last_mut() {
            p.charge = charge;
        }
        id
    }

    /// Integrate one time step, apply forces, age particles, and remove expired ones.
    pub fn update(&mut self, dt_ms: f64, forces: &[Force]) {
        let dt = dt_ms / 1000.0; // convert to seconds for physics

        // Snapshot positions for repulsion computation (avoid borrow conflicts).
        let positions: Vec<Vec2> = self.particles.iter().map(|p| p.position).collect();

        for (idx, particle) in self.particles.iter_mut().enumerate() {
            let mut accel = Vec2::zero();

            for force in forces {
                match force {
                    Force::Gravity { strength } => {
                        // F = m * g downward (positive y = down in screen space).
                        accel = accel.add(Vec2::new(0.0, *strength));
                    }

                    Force::Repulsion { radius, strength } => {
                        // Sum repulsion from every other particle within radius.
                        for (other_idx, &other_pos) in positions.iter().enumerate() {
                            if other_idx == idx {
                                continue;
                            }
                            let delta = particle.position.sub(other_pos);
                            let dist = delta.magnitude();
                            if dist > 1e-9 && dist < *radius {
                                let factor = strength * (1.0 - dist / radius) / (dist * particle.mass);
                                accel = accel.add(delta.normalize().scale(factor));
                            }
                        }
                    }

                    Force::Attraction { target, strength } => {
                        let delta = target.sub(particle.position);
                        let dist = delta.magnitude();
                        if dist > 1e-9 {
                            let factor = strength / particle.mass;
                            accel = accel.add(delta.normalize().scale(factor));
                        }
                    }

                    Force::Wind { direction, strength } => {
                        let wind = direction.normalize().scale(*strength / particle.mass);
                        accel = accel.add(wind);
                    }
                }
            }

            // Euler integration.
            particle.velocity = particle.velocity.add(accel.scale(dt));
            particle.position = particle.position.add(particle.velocity.scale(dt));
            particle.age_ms += dt_ms;
        }

        // Remove expired particles.
        self.particles.retain(|p| !p.is_expired());
    }

    /// Render all live particles into a pixel buffer.
    ///
    /// Each particle is painted as a single pixel at the closest integer
    /// coordinate.  Coordinates are mapped from world space where the canvas
    /// spans [0.0, 1.0) x [0.0, 1.0).
    pub fn render(&self, width: u32, height: u32) -> Vec<Vec<[u8; 3]>> {
        let mut buffer = vec![vec![[0u8; 3]; width as usize]; height as usize];

        for particle in &self.particles {
            let col = (particle.position.x * width as f64) as i64;
            let row = (particle.position.y * height as f64) as i64;
            if col >= 0 && col < width as i64 && row >= 0 && row < height as i64 {
                buffer[row as usize][col as usize] = particle.color;
            }
        }

        buffer
    }

    /// Number of live particles.
    pub fn particle_count(&self) -> usize {
        self.particles.len()
    }

    /// Fraction of particles still alive relative to total ever emitted.
    /// Returns 1.0 when no particles have ever been emitted.
    pub fn alive_fraction(&self) -> f64 {
        if self.next_id == 1 {
            return 1.0; // nothing emitted yet
        }
        let ever_emitted = self.next_id - 1;
        self.particles.len() as f64 / ever_emitted as f64
    }

    /// Read-only access to live particles (for inspection in tests).
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        ParticleSystem::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_system() -> ParticleSystem {
        ParticleSystem::new()
    }

    // --- Vec2 tests ---

    #[test]
    fn vec2_add() {
        let v = Vec2::new(1.0, 2.0).add(Vec2::new(3.0, 4.0));
        assert_eq!(v, Vec2::new(4.0, 6.0));
    }

    #[test]
    fn vec2_sub() {
        let v = Vec2::new(5.0, 3.0).sub(Vec2::new(2.0, 1.0));
        assert_eq!(v, Vec2::new(3.0, 2.0));
    }

    #[test]
    fn vec2_scale() {
        let v = Vec2::new(2.0, 4.0).scale(0.5);
        assert_eq!(v, Vec2::new(1.0, 2.0));
    }

    #[test]
    fn vec2_magnitude() {
        let m = Vec2::new(3.0, 4.0).magnitude();
        assert!((m - 5.0).abs() < 1e-9);
    }

    #[test]
    fn vec2_normalize_unit() {
        let n = Vec2::new(3.0, 4.0).normalize();
        assert!((n.magnitude() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn vec2_normalize_zero() {
        let n = Vec2::zero().normalize();
        assert_eq!(n, Vec2::zero());
    }

    #[test]
    fn vec2_dot() {
        let d = Vec2::new(1.0, 0.0).dot(Vec2::new(0.0, 1.0));
        assert!((d - 0.0).abs() < 1e-12);
    }

    // --- ParticleSystem tests ---

    #[test]
    fn emit_returns_unique_ids() {
        let mut sys = make_system();
        let id1 = sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 1000.0, [255, 0, 0]);
        let id2 = sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 1000.0, [0, 255, 0]);
        assert_ne!(id1, id2);
    }

    #[test]
    fn particle_count_after_emit() {
        let mut sys = make_system();
        sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 500.0, [0, 0, 0]);
        sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 500.0, [0, 0, 0]);
        assert_eq!(sys.particle_count(), 2);
    }

    #[test]
    fn particles_expire_after_lifetime() {
        let mut sys = make_system();
        sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 50.0, [0, 0, 0]);
        sys.update(100.0, &[]); // advance past lifetime
        assert_eq!(sys.particle_count(), 0);
    }

    #[test]
    fn particles_alive_within_lifetime() {
        let mut sys = make_system();
        sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 1000.0, [0, 0, 0]);
        sys.update(10.0, &[]);
        assert_eq!(sys.particle_count(), 1);
    }

    #[test]
    fn gravity_moves_particle_down() {
        let mut sys = make_system();
        sys.emit(Vec2::new(0.5, 0.5), Vec2::zero(), 1.0, 10_000.0, [255, 255, 255]);
        let forces = vec![Force::Gravity { strength: 9.8 }];
        let initial_y = sys.particles()[0].position.y;
        sys.update(16.0, &forces); // ~1 frame at 60fps
        let new_y = sys.particles()[0].position.y;
        assert!(new_y > initial_y, "particle should move down under gravity");
    }

    #[test]
    fn wind_moves_particle_in_wind_direction() {
        let mut sys = make_system();
        sys.emit(Vec2::new(0.5, 0.5), Vec2::zero(), 1.0, 10_000.0, [0, 0, 255]);
        let forces = vec![Force::Wind {
            direction: Vec2::new(1.0, 0.0),
            strength: 10.0,
        }];
        let initial_x = sys.particles()[0].position.x;
        sys.update(100.0, &forces);
        let new_x = sys.particles()[0].position.x;
        assert!(new_x > initial_x, "particle should drift right with wind");
    }

    #[test]
    fn render_returns_correct_dimensions() {
        let mut sys = make_system();
        sys.emit(Vec2::new(0.5, 0.5), Vec2::zero(), 1.0, 1000.0, [255, 0, 0]);
        let buf = sys.render(64, 64);
        assert_eq!(buf.len(), 64);
        assert_eq!(buf[0].len(), 64);
    }

    #[test]
    fn render_paints_particle_pixel() {
        let mut sys = make_system();
        let color = [200u8, 100, 50];
        sys.emit(Vec2::new(0.0, 0.0), Vec2::zero(), 1.0, 1000.0, color);
        let buf = sys.render(100, 100);
        // Particle at (0,0) world → pixel (0,0).
        assert_eq!(buf[0][0], color);
    }

    #[test]
    fn alive_fraction_after_all_expire() {
        let mut sys = make_system();
        sys.emit(Vec2::zero(), Vec2::zero(), 1.0, 10.0, [0, 0, 0]);
        sys.update(100.0, &[]);
        assert_eq!(sys.alive_fraction(), 0.0);
    }

    #[test]
    fn alive_fraction_none_emitted() {
        let sys = make_system();
        assert_eq!(sys.alive_fraction(), 1.0);
    }

    #[test]
    fn attraction_pulls_towards_target() {
        let mut sys = make_system();
        let target = Vec2::new(1.0, 0.5);
        sys.emit(Vec2::new(0.0, 0.5), Vec2::zero(), 1.0, 10_000.0, [0, 255, 0]);
        let forces = vec![Force::Attraction {
            target,
            strength: 50.0,
        }];
        let initial_x = sys.particles()[0].position.x;
        sys.update(100.0, &forces);
        let new_x = sys.particles()[0].position.x;
        assert!(new_x > initial_x, "particle should be pulled right towards target");
    }
}
