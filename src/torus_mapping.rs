//! Map 2D texture patterns onto torus and sphere surfaces.
//!
//! Provides parametric surface point generation, orthographic projection,
//! and a simple texture-sampled rasteriser for both torus and sphere.

use std::f64::consts::PI;

// ── TorusParams ───────────────────────────────────────────────────────────────

/// Parameters for a torus surface.
///
/// * `major_radius` — distance from the tube centre to the torus centre.
/// * `minor_radius` — radius of the tube.
/// * `u_steps`      — number of parameter steps around the major circle.
/// * `v_steps`      — number of parameter steps around the tube.
#[derive(Debug, Clone)]
pub struct TorusParams {
    pub major_radius: f64,
    pub minor_radius: f64,
    pub u_steps: u32,
    pub v_steps: u32,
}

impl Default for TorusParams {
    fn default() -> Self {
        Self {
            major_radius: 3.0,
            minor_radius: 1.0,
            u_steps: 64,
            v_steps: 32,
        }
    }
}

/// Compute a 3D point on a torus.
///
/// * `u` — angle around the major circle ∈ [0, 2π).
/// * `v` — angle around the tube ∈ [0, 2π).
pub fn torus_point(u: f64, v: f64, params: &TorusParams) -> (f64, f64, f64) {
    let r = params.major_radius;
    let r2 = params.minor_radius;
    let x = (r + r2 * v.cos()) * u.cos();
    let y = (r + r2 * v.cos()) * u.sin();
    let z = r2 * v.sin();
    (x, y, z)
}

// ── SphereParams ──────────────────────────────────────────────────────────────

/// Parameters for a sphere surface.
///
/// * `u_steps` — longitude steps.
/// * `v_steps` — latitude steps.
#[derive(Debug, Clone)]
pub struct SphereParams {
    pub radius: f64,
    pub u_steps: u32,
    pub v_steps: u32,
}

impl Default for SphereParams {
    fn default() -> Self {
        Self {
            radius: 3.0,
            u_steps: 64,
            v_steps: 32,
        }
    }
}

/// Compute a 3D point on a sphere.
///
/// * `u` — longitude ∈ [0, 2π).
/// * `v` — latitude ∈ [0, π] (0 = north pole, π = south pole).
pub fn sphere_point(u: f64, v: f64, params: &SphereParams) -> (f64, f64, f64) {
    let r = params.radius;
    let x = r * v.sin() * u.cos();
    let y = r * v.sin() * u.sin();
    let z = r * v.cos();
    (x, y, z)
}

// ── Projection ────────────────────────────────────────────────────────────────

/// Orthographic projection of a 3D point onto a 2D screen.
///
/// The view axis is along +Z (the camera looks in the −Z direction). Points
/// with z < 0 are considered to be "behind" the view and return `None`.
///
/// Returns pixel coordinates `(col, row)` clamped to `[0, width) × [0, height)`.
pub fn project_orthographic(
    point: (f64, f64, f64),
    width: u32,
    height: u32,
) -> Option<(usize, usize)> {
    let (x, y, z) = point;
    // Discard back-facing half.
    if z < 0.0 {
        return None;
    }
    let hw = width as f64 / 2.0;
    let hh = height as f64 / 2.0;
    let col = (x + hw).round() as i64;
    let row = (hh - y).round() as i64;
    if col < 0 || col >= width as i64 || row < 0 || row >= height as i64 {
        return None;
    }
    Some((col as usize, row as usize))
}

// ── Texture sampling helper ───────────────────────────────────────────────────

/// Sample a 2D texture (row-major `Vec<Vec<[u8;3]>>`) using normalised UV ∈ [0,1].
fn sample_texture(texture: &[Vec<[u8; 3]>], u_norm: f64, v_norm: f64) -> [u8; 3] {
    if texture.is_empty() || texture[0].is_empty() {
        return [0; 3];
    }
    let h = texture.len();
    let w = texture[0].len();
    let row = ((v_norm.rem_euclid(1.0)) * h as f64).floor() as usize % h;
    let col = ((u_norm.rem_euclid(1.0)) * w as f64).floor() as usize % w;
    texture[row][col]
}

// ── render_torus ──────────────────────────────────────────────────────────────

/// Render a texture-mapped torus using orthographic projection.
///
/// Iterates over all (u, v) parameter pairs, samples the texture at the
/// corresponding UV coordinate, and writes the colour to the screen pixel.
pub fn render_torus(
    texture: &Vec<Vec<[u8; 3]>>,
    params: &TorusParams,
    width: u32,
    height: u32,
) -> Vec<Vec<[u8; 3]>> {
    let bg = [20u8, 20, 20];
    let mut image = vec![vec![bg; width as usize]; height as usize];

    for ui in 0..params.u_steps {
        for vi in 0..params.v_steps {
            let u = 2.0 * PI * ui as f64 / params.u_steps as f64;
            let v = 2.0 * PI * vi as f64 / params.v_steps as f64;
            let point = torus_point(u, v, params);
            // Scale point to fit screen.
            let scale =
                (width.min(height) as f64) / (2.0 * (params.major_radius + params.minor_radius));
            let scaled = (point.0 * scale, point.1 * scale, point.2 * scale);
            if let Some((col, row)) = project_orthographic(scaled, width, height) {
                let u_norm = ui as f64 / params.u_steps as f64;
                let v_norm = vi as f64 / params.v_steps as f64;
                image[row][col] = sample_texture(texture, u_norm, v_norm);
            }
        }
    }
    image
}

// ── render_sphere ─────────────────────────────────────────────────────────────

/// Render a texture-mapped sphere using orthographic projection.
pub fn render_sphere(
    texture: &Vec<Vec<[u8; 3]>>,
    params: &SphereParams,
    width: u32,
    height: u32,
) -> Vec<Vec<[u8; 3]>> {
    let bg = [20u8, 20, 20];
    let mut image = vec![vec![bg; width as usize]; height as usize];

    let scale = (width.min(height) as f64) / (2.0 * params.radius);

    for ui in 0..params.u_steps {
        for vi in 0..params.v_steps {
            let u = 2.0 * PI * ui as f64 / params.u_steps as f64;
            let v = PI * vi as f64 / params.v_steps as f64;
            let point = sphere_point(u, v, params);
            let scaled = (point.0 * scale, point.1 * scale, point.2 * scale);
            if let Some((col, row)) = project_orthographic(scaled, width, height) {
                let u_norm = ui as f64 / params.u_steps as f64;
                let v_norm = vi as f64 / params.v_steps as f64;
                image[row][col] = sample_texture(texture, u_norm, v_norm);
            }
        }
    }
    image
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torus_point_at_origin_angle() {
        let params = TorusParams {
            major_radius: 3.0,
            minor_radius: 1.0,
            u_steps: 32,
            v_steps: 16,
        };
        let (x, y, z) = torus_point(0.0, 0.0, &params);
        // u=0, v=0 → x=R+r, y=0, z=0
        assert!((x - 4.0).abs() < 1e-10, "x={x}");
        assert!(y.abs() < 1e-10, "y={y}");
        assert!(z.abs() < 1e-10, "z={z}");
    }

    #[test]
    fn torus_point_distance_from_center_is_major_radius() {
        let params = TorusParams::default();
        // For v=0 (outermost equator), the xy distance from origin equals R+r.
        for i in 0..8 {
            let u = 2.0 * PI * i as f64 / 8.0;
            let (x, y, _z) = torus_point(u, 0.0, &params);
            let dist = (x * x + y * y).sqrt();
            let expected = params.major_radius + params.minor_radius;
            assert!(
                (dist - expected).abs() < 1e-9,
                "dist={dist} expected={expected}"
            );
        }
    }

    #[test]
    fn sphere_point_at_north_pole() {
        let params = SphereParams::default();
        let (x, y, z) = sphere_point(0.0, 0.0, &params);
        assert!(x.abs() < 1e-10);
        assert!(y.abs() < 1e-10);
        assert!((z - params.radius).abs() < 1e-10, "z={z}");
    }

    #[test]
    fn sphere_point_on_surface() {
        let params = SphereParams {
            radius: 2.0,
            u_steps: 16,
            v_steps: 8,
        };
        for ui in 0..8 {
            for vi in 0..4 {
                let u = 2.0 * PI * ui as f64 / 8.0;
                let v = PI * vi as f64 / 4.0;
                let (x, y, z) = sphere_point(u, v, &params);
                let r = (x * x + y * y + z * z).sqrt();
                assert!((r - params.radius).abs() < 1e-9, "r={r}");
            }
        }
    }

    #[test]
    fn orthographic_center() {
        let pt = (0.0, 0.0, 1.0);
        let result = project_orthographic(pt, 100, 100);
        assert!(result.is_some());
        let (col, row) = result.unwrap();
        assert_eq!(col, 50);
        assert_eq!(row, 50);
    }

    #[test]
    fn orthographic_behind_view_returns_none() {
        let pt = (0.0, 0.0, -1.0);
        assert!(project_orthographic(pt, 100, 100).is_none());
    }

    #[test]
    fn orthographic_out_of_bounds_returns_none() {
        let pt = (200.0, 0.0, 1.0);
        assert!(project_orthographic(pt, 100, 100).is_none());
    }

    fn checker_texture(size: usize) -> Vec<Vec<[u8; 3]>> {
        (0..size)
            .map(|r| {
                (0..size)
                    .map(|c| {
                        if (r + c) % 2 == 0 {
                            [255u8, 255, 255]
                        } else {
                            [0u8, 0, 0]
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn render_torus_dimensions() {
        let texture = checker_texture(32);
        let params = TorusParams {
            major_radius: 2.0,
            minor_radius: 0.5,
            u_steps: 16,
            v_steps: 8,
        };
        let img = render_torus(&texture, &params, 64, 64);
        assert_eq!(img.len(), 64);
        assert_eq!(img[0].len(), 64);
    }

    #[test]
    fn render_torus_paints_some_pixels() {
        let texture = checker_texture(16);
        let params = TorusParams {
            major_radius: 2.0,
            minor_radius: 0.5,
            u_steps: 64,
            v_steps: 32,
        };
        let bg = [20u8, 20, 20];
        let img = render_torus(&texture, &params, 128, 128);
        let painted = img.iter().flatten().any(|&px| px != bg);
        assert!(painted, "torus render produced blank image");
    }

    #[test]
    fn render_sphere_dimensions() {
        let texture = checker_texture(32);
        let params = SphereParams {
            radius: 2.0,
            u_steps: 16,
            v_steps: 8,
        };
        let img = render_sphere(&texture, &params, 64, 64);
        assert_eq!(img.len(), 64);
        assert_eq!(img[0].len(), 64);
    }

    #[test]
    fn render_sphere_paints_some_pixels() {
        let texture = checker_texture(16);
        let params = SphereParams {
            radius: 2.0,
            u_steps: 64,
            v_steps: 32,
        };
        let bg = [20u8, 20, 20];
        let img = render_sphere(&texture, &params, 128, 128);
        let painted = img.iter().flatten().any(|&px| px != bg);
        assert!(painted, "sphere render produced blank image");
    }

    #[test]
    fn sample_texture_wraps() {
        let tex = checker_texture(4);
        // u_norm=1.0 should wrap to 0.
        let c1 = sample_texture(&tex, 0.0, 0.0);
        let c2 = sample_texture(&tex, 1.0, 0.0);
        assert_eq!(c1, c2);
    }
}
