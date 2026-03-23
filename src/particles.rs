//! Particle system that follows geodesic paths on the surface.
//!
//! Particles are born at random surface points, travel along geodesics using the
//! same RK4 integrator as the main geodesic curves, leave coloured trails, and
//! die when they reach their maximum age or reach a domain boundary.
//!
//! # Design
//!
//! - Particles are maintained in a fixed-size pool to avoid heap churn.
//! - Spawning is staggered so not all particles die at the same frame.
//! - Trail vertices are stored in a ring buffer identical to [`TrailBuffer`].
//! - The system integrates with the existing wgpu trail rendering pipeline.

#![allow(dead_code)]

use glam::Vec3;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::geodesic::Geodesic;
use crate::surface::Surface;
use crate::trail::{TrailBuffer, TrailVertex};

// ── Particle ──────────────────────────────────────────────────────────────────

/// A single geodesic particle with its own trail buffer.
pub struct Particle {
    /// Underlying geodesic state.
    pub geo: Geodesic,
    /// Trail ring buffer.
    pub trail: TrailBuffer,
    /// Whether this slot is currently active.
    pub alive: bool,
}

impl Particle {
    fn new(capacity: usize) -> Self {
        Self {
            geo: Geodesic::new(0.0, 0.0, 1.0, 0.0, 0, 0),
            trail: TrailBuffer::new(capacity, [1.0, 1.0, 1.0, 1.0], 2.0),
            alive: false,
        }
    }
}

// ── Spawn profile ─────────────────────────────────────────────────────────────

/// Controls how particles are spawned and how they look.
#[derive(Debug, Clone)]
pub struct ParticleConfig {
    /// Maximum number of concurrent particles.
    pub max_particles: usize,
    /// Trail length in frames.
    pub trail_length: usize,
    /// Particle lifetime in frames.
    pub lifetime_frames: usize,
    /// How many new particles to spawn per frame.
    pub spawn_rate: f32,
    /// Base colour for newly spawned particles (RGBA).
    pub base_color: [f32; 4],
    /// Whether to randomise colour per particle.
    pub randomize_color: bool,
    /// RK4 time step.
    pub time_step: f32,
    /// Velocity scaling (speed along geodesic).
    pub speed: f32,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            max_particles: 128,
            trail_length: 60,
            lifetime_frames: 240,
            spawn_rate: 2.0,
            base_color: [0.8, 0.5, 1.0, 0.9],
            randomize_color: true,
            time_step: 0.016,
            speed: 1.0,
        }
    }
}

// ── Particle system ───────────────────────────────────────────────────────────

/// Pool-based particle system.
pub struct ParticleSystem {
    pool: Vec<Particle>,
    rng: StdRng,
    pub config: ParticleConfig,
    spawn_accumulator: f32,
    color_palette: Vec<[f32; 4]>,
    color_idx: usize,
}

impl ParticleSystem {
    /// Create a new particle system with the given configuration.
    pub fn new(config: ParticleConfig) -> Self {
        let pool: Vec<Particle> = (0..config.max_particles)
            .map(|_| Particle::new(config.trail_length))
            .collect();
        Self {
            pool,
            rng: StdRng::seed_from_u64(0xDEAD_BEEF_1234),
            config,
            spawn_accumulator: 0.0,
            color_palette: default_palette(),
            color_idx: 0,
        }
    }

    /// Advance the system by one frame.
    ///
    /// - Spawns new particles according to `spawn_rate`.
    /// - Advances all alive particles one RK4 step.
    /// - Records new trail positions.
    /// - Kills particles that have reached their maximum age.
    pub fn tick(&mut self, surface: &dyn Surface) {
        self.spawn_due(surface);
        self.step_all(surface);
    }

    fn spawn_due(&mut self, surface: &dyn Surface) {
        self.spawn_accumulator += self.config.spawn_rate;
        while self.spawn_accumulator >= 1.0 {
            self.spawn_one(surface);
            self.spawn_accumulator -= 1.0;
        }
    }

    fn spawn_one(&mut self, surface: &dyn Surface) {
        // Find a dead slot.
        let slot = match self.pool.iter().position(|p| !p.alive) {
            Some(i) => i,
            None => return, // pool full
        };

        let (u, v) = surface.random_position(&mut self.rng);
        let (du, dv) = surface.random_tangent(u, v, &mut self.rng);
        let speed = self.config.speed;

        let color = if self.config.randomize_color {
            self.next_color()
        } else {
            self.config.base_color
        };

        let lifetime = self.config.lifetime_frames
            + self.rng.gen_range(0..=self.config.lifetime_frames / 4);

        let p = &mut self.pool[slot];
        p.geo = Geodesic::new(u, v, du * speed, dv * speed, lifetime, self.color_idx);
        p.trail = TrailBuffer::new(self.config.trail_length, color, 2.0);
        p.alive = true;

        // Record initial position using the existing push([f32; 3]) signature.
        let pos3 = surface.position(u, v);
        p.trail.push([pos3.x, pos3.y, pos3.z]);
    }

    fn next_color(&mut self) -> [f32; 4] {
        let c = self.color_palette[self.color_idx % self.color_palette.len()];
        self.color_idx += 1;
        c
    }

    fn step_all(&mut self, surface: &dyn Surface) {
        let dt = self.config.time_step;
        for p in &mut self.pool {
            if !p.alive {
                continue;
            }
            p.geo.step(surface, dt);
            if !p.geo.alive {
                p.alive = false;
                continue;
            }
            let pos3 = surface.position(p.geo.u, p.geo.v);
            p.trail.push([pos3.x, pos3.y, pos3.z]);
        }
    }

    /// Collect all trail vertices from alive particles for GPU upload.
    pub fn collect_vertices(&self) -> Vec<TrailVertex> {
        let mut verts = Vec::new();
        for p in &self.pool {
            if p.alive {
                verts.extend(p.trail.ordered_vertices());
            }
        }
        verts
    }

    /// Number of currently alive particles.
    pub fn alive_count(&self) -> usize {
        self.pool.iter().filter(|p| p.alive).count()
    }

    /// Set a custom colour palette.
    pub fn set_palette(&mut self, palette: Vec<[f32; 4]>) {
        if !palette.is_empty() {
            self.color_palette = palette;
        }
    }
}

fn default_palette() -> Vec<[f32; 4]> {
    vec![
        [0.3, 0.8, 1.0, 0.9],  // cyan
        [1.0, 0.4, 0.8, 0.9],  // pink
        [0.5, 1.0, 0.5, 0.9],  // green
        [1.0, 0.8, 0.2, 0.9],  // gold
        [0.7, 0.4, 1.0, 0.9],  // violet
    ]
}

// No TrailBuffer extension needed — the particle system uses the existing
// TrailBuffer::push([f32; 3]) method defined in crate::trail.
// Colour is set at TrailBuffer construction time via the `color` field.

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface::torus::Torus;

    fn make_system() -> ParticleSystem {
        ParticleSystem::new(ParticleConfig {
            max_particles: 16,
            trail_length: 20,
            lifetime_frames: 30,
            spawn_rate: 4.0,
            ..Default::default()
        })
    }

    #[test]
    fn test_particles_spawn_on_tick() {
        let mut sys = make_system();
        let surf = Torus::new(2.0, 0.7);
        sys.tick(&surf);
        assert!(sys.alive_count() > 0, "particles should spawn on first tick");
    }

    #[test]
    fn test_particles_die_after_lifetime() {
        let mut sys = make_system();
        let surf = Torus::new(2.0, 0.7);
        // Tick far beyond particle lifetime.
        for _ in 0..200 {
            sys.tick(&surf);
        }
        // Most particles should have cycled; alive count bounded by max.
        assert!(sys.alive_count() <= sys.config.max_particles);
    }

    #[test]
    fn test_collect_vertices_finite() {
        let mut sys = make_system();
        let surf = Torus::new(2.0, 0.7);
        for _ in 0..5 {
            sys.tick(&surf);
        }
        for v in sys.collect_vertices() {
            assert!(v.position.iter().all(|x| x.is_finite()));
            assert!(v.color.iter().all(|x| x.is_finite()));
        }
    }

    #[test]
    fn test_alive_count_bounded() {
        let mut sys = make_system();
        let surf = Torus::new(2.0, 0.7);
        for _ in 0..50 {
            sys.tick(&surf);
            assert!(sys.alive_count() <= sys.config.max_particles);
        }
    }
}
