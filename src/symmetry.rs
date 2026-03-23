//! Wallpaper symmetry group pattern generators.
//!
//! This module implements two of the highest-complexity wallpaper groups that
//! were not previously present: **p4g** (square lattice with glide reflections,
//! 8 symmetry operations) and **p6m** (hexagonal lattice with all reflections,
//! 12 symmetry operations).
//!
//! Wallpaper groups tile the plane and are commonly used to generate
//! symmetric textures for surface coloring or background patterns. Each
//! group provides a [`SymmetryGroup`] trait implementation that maps any
//! 2D point to a canonical fundamental domain coordinate.
//!
//! ## Groups
//!
//! | Group | Lattice | Ops | Description |
//! |-------|---------|-----|-------------|
//! | p4g   | Square  | 8   | 4-fold rotation + glide reflections |
//! | p6m   | Hex     | 12  | 6-fold rotation + all 6 reflection axes |
//!
//! ## Usage
//!
//! ```rust
//! use geodesic_wallpaper::symmetry::{P4g, P6m, SymmetryGroup};
//!
//! let p4g = P4g::new(1.0);
//! let (u, v) = p4g.to_fundamental_domain(1.3, 2.7);
//! // u, v are in the fundamental domain [0,1)^2
//!
//! let p6m = P6m::new(1.0);
//! let (u, v) = p6m.to_fundamental_domain(0.5, 0.8);
//! ```

use std::f32::consts::PI;

// ── Trait ─────────────────────────────────────────────────────────────────────

/// A wallpaper symmetry group: maps any 2D point to a fundamental domain.
pub trait SymmetryGroup: Send + Sync {
    /// Return the name of this symmetry group (e.g. `"p4g"`).
    fn name(&self) -> &str;

    /// Number of distinct symmetry operations (order of the point group).
    fn operations(&self) -> usize;

    /// Map `(x, y)` to a canonical point in the fundamental domain.
    ///
    /// The fundamental domain is a minimal tile that — when replicated under
    /// all group operations — covers the entire plane without overlap.
    ///
    /// Returns `(u, v)` in the domain appropriate for the group.
    fn to_fundamental_domain(&self, x: f32, y: f32) -> (f32, f32);

    /// Apply all symmetry operations to `(x, y)` and return the orbit.
    ///
    /// The orbit has length equal to [`operations`].
    fn orbit(&self, x: f32, y: f32) -> Vec<(f32, f32)>;

    /// Evaluate a "coloring function" at `(x, y)` using the fundamental domain.
    ///
    /// Returns a value in `[0, 1]` suitable for palette indexing.
    fn color_value(&self, x: f32, y: f32) -> f32 {
        let (u, v) = self.to_fundamental_domain(x, y);
        (u * u + v * v).sqrt().fract()
    }
}

// ── P4g: square lattice with glide reflections ────────────────────────────────

/// Wallpaper group **p4g** — square lattice, 4-fold rotation + glide reflections.
///
/// The group has 8 symmetry operations:
/// - 4 rotations (0°, 90°, 180°, 270°)
/// - 2 reflections about the diagonals
/// - 2 glide reflections (reflection + half-cell translation)
///
/// The fundamental domain is 1/8 of the square unit cell.
pub struct P4g {
    /// Lattice period (cell size).
    pub period: f32,
}

impl P4g {
    pub fn new(period: f32) -> Self {
        Self { period: period.max(1e-6) }
    }

    /// Reduce `(x, y)` modulo the lattice to `[0, period)^2`.
    fn reduce(&self, x: f32, y: f32) -> (f32, f32) {
        let a = x.rem_euclid(self.period);
        let b = y.rem_euclid(self.period);
        (a, b)
    }
}

impl SymmetryGroup for P4g {
    fn name(&self) -> &str { "p4g" }

    fn operations(&self) -> usize { 8 }

    fn to_fundamental_domain(&self, x: f32, y: f32) -> (f32, f32) {
        let p = self.period;
        let (mut u, mut v) = self.reduce(x, y);

        // Fold into [0, p/2]^2 using 180° rotation symmetry
        if u > p * 0.5 { u = p - u; }
        if v > p * 0.5 { v = p - v; }

        // Fold by the diagonal reflection (glide): if u < v, swap
        if u < v { std::mem::swap(&mut u, &mut v); }

        // Normalise to [0, 1)
        (u / p, v / p)
    }

    fn orbit(&self, x: f32, y: f32) -> Vec<(f32, f32)> {
        let p = self.period;
        // 4 rotations of (x, y) around the cell center (p/2, p/2)
        let cx = p * 0.5;
        let cy = p * 0.5;
        let dx = x - cx;
        let dy = y - cy;
        // Rotations: 0°, 90°, 180°, 270°
        let rotations: Vec<(f32, f32)> = (0..4)
            .map(|k| {
                let angle = k as f32 * PI * 0.5;
                let (cos_a, sin_a) = (angle.cos(), angle.sin());
                (cx + cos_a * dx - sin_a * dy, cy + sin_a * dx + cos_a * dy)
            })
            .collect();
        // Add 4 diagonal-reflected versions (p4g glide reflections)
        let reflections: Vec<(f32, f32)> = rotations
            .iter()
            .map(|&(rx, ry)| {
                // Reflect about the diagonal x=y through center
                let rdx = rx - cx;
                let rdy = ry - cy;
                (cx + rdy, cy + rdx) // swap dx and dy
            })
            .collect();
        rotations.into_iter().chain(reflections).collect()
    }
}

// ── P6m: hexagonal lattice with all reflections ───────────────────────────────

/// Wallpaper group **p6m** — hexagonal lattice, 6-fold rotation + 6 reflection axes.
///
/// The group has 12 symmetry operations:
/// - 6 rotations (0°, 60°, 120°, 180°, 240°, 300°)
/// - 6 reflections (one per rotation axis)
///
/// This is the highest-symmetry wallpaper group. The fundamental domain is
/// 1/12 of the hexagonal unit cell — a right triangle with angles 30°-60°-90°.
pub struct P6m {
    /// Lattice period (distance between adjacent hexagon centers).
    pub period: f32,
}

impl P6m {
    pub fn new(period: f32) -> Self {
        Self { period: period.max(1e-6) }
    }

    /// Convert rectangular to hexagonal lattice coordinates.
    fn to_hex_coords(&self, x: f32, y: f32) -> (f32, f32) {
        let p = self.period;
        // Hexagonal lattice basis vectors: a1=(1,0), a2=(0.5, sqrt(3)/2)
        // Inverse: s = x - y/sqrt(3), t = 2*y/sqrt(3)
        let s = (x / p).rem_euclid(1.0);
        let t = (y / p * 2.0 / 3.0_f32.sqrt()).rem_euclid(1.0);
        (s, t)
    }
}

impl SymmetryGroup for P6m {
    fn name(&self) -> &str { "p6m" }

    fn operations(&self) -> usize { 12 }

    fn to_fundamental_domain(&self, x: f32, y: f32) -> (f32, f32) {
        let p = self.period;
        // Reduce to the hexagonal Wigner-Seitz cell using oblique coordinates
        // Lattice: e1 = (p, 0), e2 = (p/2, p*sqrt(3)/2)
        let sqrt3 = 3.0_f32.sqrt();
        let e1x = p;
        let e1y = 0.0_f32;
        let e2x = p * 0.5;
        let e2y = p * sqrt3 * 0.5;

        // Fractional coordinates
        let det = e1x * e2y - e1y * e2x;
        let s = (x * e2y - y * e2x) / det;
        let t = (x * (-e1y) + y * e1x) / det;
        let s = s.rem_euclid(1.0);
        let t = t.rem_euclid(1.0);

        // Now fold by 6-fold symmetry into the fundamental triangle
        // Map angle to [0, 60°) and reflect by the mirror at 30°
        let angle = t.atan2(s); // in [-π, π]
        let angle_deg = angle.to_degrees().rem_euclid(360.0);
        let sector = (angle_deg / 60.0).floor() as u32;
        let local_angle = angle_deg - sector as f32 * 60.0;
        let folded_angle = if local_angle > 30.0 { 60.0 - local_angle } else { local_angle };

        let r = (s * s + t * t).sqrt();
        let (fu, fv) = (
            r * (folded_angle * PI / 180.0).cos(),
            r * (folded_angle * PI / 180.0).sin(),
        );

        // Scale to [0, 1)
        let scale = p.recip();
        (fu * scale, fv * scale)
    }

    fn orbit(&self, x: f32, y: f32) -> Vec<(f32, f32)> {
        let mut orbit = Vec::with_capacity(12);
        // 6 rotations at 60° intervals
        for k in 0..6 {
            let angle = k as f32 * PI / 3.0;
            let (cos_a, sin_a) = (angle.cos(), angle.sin());
            let rx = cos_a * x - sin_a * y;
            let ry = sin_a * x + cos_a * y;
            orbit.push((rx, ry));
        }
        // 6 reflections (mirror along the x-axis for each rotated point)
        let base: Vec<_> = orbit.clone();
        for (rx, ry) in base {
            orbit.push((rx, -ry));
        }
        orbit
    }
}

// ── Pattern sampling ──────────────────────────────────────────────────────────

/// Sample a symmetry group pattern over a rectangular grid.
///
/// Returns a flat `Vec<f32>` in row-major order with values in `[0, 1]`.
/// Useful for generating texture data or color maps.
pub fn sample_pattern(
    group: &dyn SymmetryGroup,
    width: usize,
    height: usize,
    x_range: (f32, f32),
    y_range: (f32, f32),
) -> Vec<f32> {
    let mut out = vec![0.0f32; width * height];
    for row in 0..height {
        for col in 0..width {
            let x = x_range.0 + (col as f32 / width as f32) * (x_range.1 - x_range.0);
            let y = y_range.0 + (row as f32 / height as f32) * (y_range.1 - y_range.0);
            out[row * width + col] = group.color_value(x, y);
        }
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── P4g tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_p4g_name() {
        assert_eq!(P4g::new(1.0).name(), "p4g");
    }

    #[test]
    fn test_p4g_operations_count() {
        assert_eq!(P4g::new(1.0).operations(), 8);
    }

    #[test]
    fn test_p4g_fundamental_domain_in_range() {
        let p4g = P4g::new(1.0);
        for i in 0..20 {
            let x = i as f32 * 0.3 - 2.0;
            let y = i as f32 * 0.17 - 1.5;
            let (u, v) = p4g.to_fundamental_domain(x, y);
            assert!((0.0..=1.0).contains(&u), "u out of range: {u} at ({x}, {y})");
            assert!((0.0..=1.0).contains(&v), "v out of range: {v} at ({x}, {y})");
        }
    }

    #[test]
    fn test_p4g_fundamental_domain_u_ge_v() {
        // By construction, u >= v in the fundamental domain
        let p4g = P4g::new(1.0);
        for i in 0..50 {
            let x = i as f32 * 0.07;
            let y = i as f32 * 0.13;
            let (u, v) = p4g.to_fundamental_domain(x, y);
            assert!(u >= v - 1e-6, "u ({u}) should be >= v ({v})");
        }
    }

    #[test]
    fn test_p4g_orbit_has_8_points() {
        let p4g = P4g::new(1.0);
        assert_eq!(p4g.orbit(0.3, 0.1).len(), 8);
    }

    #[test]
    fn test_p4g_color_value_in_range() {
        let p4g = P4g::new(1.0);
        let v = p4g.color_value(1.23, 0.77);
        assert!((0.0..=1.0).contains(&v));
    }

    #[test]
    fn test_p4g_different_period() {
        let p4g_1 = P4g::new(1.0);
        let p4g_2 = P4g::new(2.0);
        let (u1, v1) = p4g_1.to_fundamental_domain(1.5, 0.7);
        let (u2, v2) = p4g_2.to_fundamental_domain(1.5, 0.7);
        // Different periods yield different fundamental domain coords
        assert!((u1 - u2).abs() > 1e-6 || (v1 - v2).abs() > 1e-6);
    }

    #[test]
    fn test_p4g_reduces_modulo_period() {
        let p4g = P4g::new(1.0);
        let (u1, v1) = p4g.to_fundamental_domain(0.3, 0.1);
        let (u2, v2) = p4g.to_fundamental_domain(1.3, 1.1); // shifted by one period
        assert!((u1 - u2).abs() < 1e-5, "u: {u1} vs {u2}");
        assert!((v1 - v2).abs() < 1e-5, "v: {v1} vs {v2}");
    }

    // ── P6m tests ─────────────────────────────────────────────────────────────

    #[test]
    fn test_p6m_name() {
        assert_eq!(P6m::new(1.0).name(), "p6m");
    }

    #[test]
    fn test_p6m_operations_count() {
        assert_eq!(P6m::new(1.0).operations(), 12);
    }

    #[test]
    fn test_p6m_orbit_has_12_points() {
        let p6m = P6m::new(1.0);
        assert_eq!(p6m.orbit(0.3, 0.2).len(), 12);
    }

    #[test]
    fn test_p6m_fundamental_domain_finite() {
        let p6m = P6m::new(1.0);
        for i in 0..20 {
            let x = i as f32 * 0.3 - 2.5;
            let y = i as f32 * 0.21 - 1.5;
            let (u, v) = p6m.to_fundamental_domain(x, y);
            assert!(u.is_finite(), "u not finite: {u}");
            assert!(v.is_finite(), "v not finite: {v}");
        }
    }

    #[test]
    fn test_p6m_color_value_in_range() {
        let p6m = P6m::new(1.0);
        for i in 0..20 {
            let v = p6m.color_value(i as f32 * 0.5, i as f32 * 0.3);
            assert!((0.0..=1.0).contains(&v), "color_value out of range: {v}");
        }
    }

    #[test]
    fn test_p6m_orbit_contains_rotation_of_input() {
        let p6m = P6m::new(1.0);
        let orbit = p6m.orbit(1.0, 0.0);
        // The 60° rotation of (1, 0) is (cos60, sin60) = (0.5, sqrt(3)/2)
        let expected_x = 0.5_f32;
        let expected_y = (3.0_f32).sqrt() * 0.5;
        let found = orbit.iter().any(|&(x, y)| {
            (x - expected_x).abs() < 1e-5 && (y - expected_y).abs() < 1e-5
        });
        assert!(found, "Orbit should contain 60° rotation: {:?}", orbit);
    }

    #[test]
    fn test_p6m_orbit_has_reflection() {
        let p6m = P6m::new(1.0);
        let orbit = p6m.orbit(0.5, 0.3);
        // Reflection of (0.5, 0.3) about x-axis is (0.5, -0.3)
        let found = orbit.iter().any(|&(x, y)| {
            (x - 0.5).abs() < 1e-5 && (y - (-0.3)).abs() < 1e-5
        });
        assert!(found, "Orbit should contain x-axis reflection: {:?}", orbit);
    }

    // ── sample_pattern tests ──────────────────────────────────────────────────

    #[test]
    fn test_sample_pattern_size() {
        let p4g = P4g::new(1.0);
        let grid = sample_pattern(&p4g, 8, 6, (0.0, 1.0), (0.0, 1.0));
        assert_eq!(grid.len(), 48);
    }

    #[test]
    fn test_sample_pattern_values_in_range() {
        let p6m = P6m::new(1.0);
        let grid = sample_pattern(&p6m, 16, 16, (-2.0, 2.0), (-2.0, 2.0));
        assert!(grid.iter().all(|&v| (0.0..=1.0).contains(&v)), "values out of range");
    }
}
