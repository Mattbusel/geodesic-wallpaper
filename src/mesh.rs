//! 3D mesh with orthographic and perspective projection.
//!
//! Provides named mesh constructors (tetrahedron, cube, icosahedron, geodesic
//! sphere), Y-axis rotation/scale transform, two projection modes, and a
//! Bresenham wireframe rasterizer.

use std::f64::consts::PI;

// ── Vec3 ──────────────────────────────────────────────────────────────────────

/// Three-dimensional vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }

    pub fn zero() -> Self {
        Vec3::new(0.0, 0.0, 0.0)
    }

    pub fn dot(&self, other: &Vec3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn cross(&self, other: &Vec3) -> Vec3 {
        Vec3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    pub fn length(&self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalize(&self) -> Vec3 {
        let len = self.length();
        if len < 1e-12 {
            return Vec3::zero();
        }
        self.scale(1.0 / len)
    }

    pub fn add(&self, other: &Vec3) -> Vec3 {
        Vec3::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    pub fn sub(&self, other: &Vec3) -> Vec3 {
        Vec3::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    pub fn scale(&self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }

    pub fn midpoint(&self, other: &Vec3) -> Vec3 {
        self.add(other).scale(0.5)
    }
}

// ── Vertex ────────────────────────────────────────────────────────────────────

/// A mesh vertex with position, smooth normal, and UV texture coordinates.
#[derive(Debug, Clone)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: (f64, f64),
}

// ── Triangle ─────────────────────────────────────────────────────────────────

/// A triangle referencing three indices into a [`Mesh`]'s vertex list.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub a: usize,
    pub b: usize,
    pub c: usize,
}

// ── Mesh ─────────────────────────────────────────────────────────────────────

/// A 3-D mesh composed of vertices and indexed triangles.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<Triangle>,
}

impl Mesh {
    fn vertex(pos: Vec3) -> Vertex {
        let normal = pos.normalize();
        let uv = (
            0.5 + pos.z.atan2(pos.x) / (2.0 * PI),
            0.5 - pos.y.asin() / PI,
        );
        Vertex { position: pos, normal, uv }
    }

    /// Regular tetrahedron inscribed in the unit sphere.
    pub fn tetrahedron() -> Self {
        let a = Vec3::new(0.0, 1.0, 0.0);
        let b = Vec3::new((8.0f64 / 9.0).sqrt(), -1.0 / 3.0, 0.0);
        let c = Vec3::new(-(2.0f64 / 9.0).sqrt(), -1.0 / 3.0, (2.0f64 / 3.0).sqrt());
        let d = Vec3::new(-(2.0f64 / 9.0).sqrt(), -1.0 / 3.0, -(2.0f64 / 3.0).sqrt());
        let vertices = vec![
            Self::vertex(a),
            Self::vertex(b),
            Self::vertex(c),
            Self::vertex(d),
        ];
        let triangles = vec![
            Triangle { a: 0, b: 1, c: 2 },
            Triangle { a: 0, b: 2, c: 3 },
            Triangle { a: 0, b: 3, c: 1 },
            Triangle { a: 1, b: 3, c: 2 },
        ];
        Mesh { vertices, triangles }
    }

    /// Axis-aligned unit cube (8 vertices, 12 triangles).
    pub fn cube() -> Self {
        let s = 0.5f64;
        let positions = vec![
            Vec3::new(-s, -s, -s), // 0
            Vec3::new( s, -s, -s), // 1
            Vec3::new( s,  s, -s), // 2
            Vec3::new(-s,  s, -s), // 3
            Vec3::new(-s, -s,  s), // 4
            Vec3::new( s, -s,  s), // 5
            Vec3::new( s,  s,  s), // 6
            Vec3::new(-s,  s,  s), // 7
        ];
        let vertices = positions.iter().map(|&p| Self::vertex(p)).collect();
        let triangles = vec![
            // Front (-z)
            Triangle { a: 0, b: 2, c: 1 }, Triangle { a: 0, b: 3, c: 2 },
            // Back (+z)
            Triangle { a: 4, b: 5, c: 6 }, Triangle { a: 4, b: 6, c: 7 },
            // Left (-x)
            Triangle { a: 0, b: 7, c: 3 }, Triangle { a: 0, b: 4, c: 7 },
            // Right (+x)
            Triangle { a: 1, b: 2, c: 6 }, Triangle { a: 1, b: 6, c: 5 },
            // Bottom (-y)
            Triangle { a: 0, b: 1, c: 5 }, Triangle { a: 0, b: 5, c: 4 },
            // Top (+y)
            Triangle { a: 3, b: 6, c: 2 }, Triangle { a: 3, b: 7, c: 6 },
        ];
        Mesh { vertices, triangles }
    }

    /// Regular icosahedron inscribed in the unit sphere (12 vertices, 20 faces).
    pub fn icosahedron() -> Self {
        let phi = (1.0 + 5.0f64.sqrt()) / 2.0;
        let norm = (1.0 + phi * phi).sqrt();
        let positions: Vec<Vec3> = vec![
            Vec3::new(-1.0,  phi,  0.0),
            Vec3::new( 1.0,  phi,  0.0),
            Vec3::new(-1.0, -phi,  0.0),
            Vec3::new( 1.0, -phi,  0.0),
            Vec3::new( 0.0, -1.0,  phi),
            Vec3::new( 0.0,  1.0,  phi),
            Vec3::new( 0.0, -1.0, -phi),
            Vec3::new( 0.0,  1.0, -phi),
            Vec3::new( phi,  0.0, -1.0),
            Vec3::new( phi,  0.0,  1.0),
            Vec3::new(-phi,  0.0, -1.0),
            Vec3::new(-phi,  0.0,  1.0),
        ]
        .into_iter()
        .map(|v| v.scale(1.0 / norm))
        .collect();

        let vertices = positions.iter().map(|&p| Self::vertex(p)).collect();
        let triangles = vec![
            Triangle { a: 0, b: 11, c: 5 },
            Triangle { a: 0, b:  5, c: 1 },
            Triangle { a: 0, b:  1, c: 7 },
            Triangle { a: 0, b:  7, c: 10 },
            Triangle { a: 0, b: 10, c: 11 },
            Triangle { a: 1, b:  5, c: 9 },
            Triangle { a: 5, b: 11, c: 4 },
            Triangle { a: 11, b: 10, c: 2 },
            Triangle { a: 10, b: 7, c: 6 },
            Triangle { a: 7, b:  1, c: 8 },
            Triangle { a: 3, b:  9, c: 4 },
            Triangle { a: 3, b:  4, c: 2 },
            Triangle { a: 3, b:  2, c: 6 },
            Triangle { a: 3, b:  6, c: 8 },
            Triangle { a: 3, b:  8, c: 9 },
            Triangle { a: 4, b:  9, c: 5 },
            Triangle { a: 2, b:  4, c: 11 },
            Triangle { a: 6, b:  2, c: 10 },
            Triangle { a: 8, b:  6, c: 7 },
            Triangle { a: 9, b:  8, c: 1 },
        ];
        Mesh { vertices, triangles }
    }

    /// Geodesic sphere: icosahedron with `subdivisions` levels of midpoint subdivision.
    pub fn geodesic_sphere(subdivisions: usize) -> Self {
        let mut mesh = Self::icosahedron();
        for _ in 0..subdivisions {
            mesh = Self::subdivide(mesh);
        }
        mesh
    }

    /// One level of Loop-style midpoint subdivision projected to the unit sphere.
    fn subdivide(mesh: Mesh) -> Mesh {
        let mut positions: Vec<Vec3> = mesh.vertices.iter().map(|v| v.position).collect();
        let mut triangles: Vec<Triangle> = Vec::new();
        let mut midpoint_cache: std::collections::HashMap<(usize, usize), usize> =
            std::collections::HashMap::new();

        let mut get_mid = |a: usize, b: usize, pos: &mut Vec<Vec3>| -> usize {
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&idx) = midpoint_cache.get(&key) {
                return idx;
            }
            let mid = pos[a].midpoint(&pos[b]).normalize();
            let idx = pos.len();
            pos.push(mid);
            midpoint_cache.insert(key, idx);
            idx
        };

        for tri in &mesh.triangles {
            let ab = get_mid(tri.a, tri.b, &mut positions);
            let bc = get_mid(tri.b, tri.c, &mut positions);
            let ca = get_mid(tri.c, tri.a, &mut positions);
            triangles.push(Triangle { a: tri.a, b: ab, c: ca });
            triangles.push(Triangle { a: tri.b, b: bc, c: ab });
            triangles.push(Triangle { a: tri.c, b: ca, c: bc });
            triangles.push(Triangle { a: ab, b: bc, c: ca });
        }

        let vertices = positions.iter().map(|&p| Self::vertex(p)).collect();
        Mesh { vertices, triangles }
    }

    /// Rotate all vertices around the Y axis by `rotation_y` radians, then scale.
    pub fn transform(&self, rotation_y: f64, scale: f64) -> Mesh {
        let cos_y = rotation_y.cos();
        let sin_y = rotation_y.sin();
        let vertices = self
            .vertices
            .iter()
            .map(|v| {
                let p = &v.position;
                let x = p.x * cos_y + p.z * sin_y;
                let y = p.y;
                let z = -p.x * sin_y + p.z * cos_y;
                let pos = Vec3::new(x * scale, y * scale, z * scale);
                Self::vertex(pos)
            })
            .collect();
        Mesh { vertices, triangles: self.triangles.clone() }
    }
}

// ── Camera ────────────────────────────────────────────────────────────────────

/// Pinhole camera for perspective projection.
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    /// Vertical field of view in degrees.
    pub fov_deg: f64,
    /// Near clipping plane distance.
    pub near: f64,
    /// Far clipping plane distance.
    pub far: f64,
}

impl Camera {
    pub fn new(position: Vec3, target: Vec3, fov_deg: f64) -> Self {
        Camera { position, target, fov_deg, near: 0.01, far: 1000.0 }
    }
}

// ── Projections ───────────────────────────────────────────────────────────────

/// Orthographic projection: drop Z, scale and centre.
///
/// Returns `None` if the projected point is outside the screen.
pub fn project_orthographic(
    vertex: &Vec3,
    width: usize,
    height: usize,
    scale: f64,
) -> Option<(i32, i32)> {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let px = (vertex.x * scale + cx).round() as i32;
    let py = (cy - vertex.y * scale).round() as i32;
    if px < 0 || px >= width as i32 || py < 0 || py >= height as i32 {
        None
    } else {
        Some((px, py))
    }
}

/// Perspective projection using a pinhole camera model.
///
/// Returns `None` if the vertex is behind or on the camera's near plane.
pub fn project_perspective(
    vertex: &Vec3,
    camera: &Camera,
    width: usize,
    height: usize,
) -> Option<(i32, i32)> {
    // Camera coordinate system.
    let forward = camera.target.sub(&camera.position).normalize();
    let world_up = Vec3::new(0.0, 1.0, 0.0);
    let right = forward.cross(&world_up).normalize();
    let up = right.cross(&forward).normalize();

    // Transform vertex to camera space.
    let rel = vertex.sub(&camera.position);
    let cam_x = rel.dot(&right);
    let cam_y = rel.dot(&up);
    let cam_z = rel.dot(&forward);

    if cam_z <= camera.near {
        return None;
    }

    let fov_rad = camera.fov_deg.to_radians();
    let aspect = width as f64 / height as f64;
    let half_h = (fov_rad / 2.0).tan();
    let half_w = half_h * aspect;

    let ndc_x = (cam_x / (cam_z * half_w)).clamp(-1.0, 1.0);
    let ndc_y = (cam_y / (cam_z * half_h)).clamp(-1.0, 1.0);

    let px = ((ndc_x + 1.0) / 2.0 * width as f64).round() as i32;
    let py = ((1.0 - ndc_y) / 2.0 * height as f64).round() as i32;

    if px < 0 || px >= width as i32 || py < 0 || py >= height as i32 {
        None
    } else {
        Some((px, py))
    }
}

// ── Rasterizer ────────────────────────────────────────────────────────────────

/// Draw all triangle edges of `mesh` using Bresenham's line algorithm.
///
/// Returns a flat RGB u8 vector of size `width * height * 3`.
pub fn rasterize_wireframe(
    mesh: &Mesh,
    camera: &Camera,
    width: usize,
    height: usize,
    color: (u8, u8, u8),
) -> Vec<u8> {
    let mut pixels = vec![0u8; width * height * 3];

    let mut set_pixel = |x: i32, y: i32| {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let idx = (y as usize * width + x as usize) * 3;
            pixels[idx] = color.0;
            pixels[idx + 1] = color.1;
            pixels[idx + 2] = color.2;
        }
    };

    let projected: Vec<Option<(i32, i32)>> = mesh
        .vertices
        .iter()
        .map(|v| project_perspective(&v.position, camera, width, height))
        .collect();

    for tri in &mesh.triangles {
        let edges = [(tri.a, tri.b), (tri.b, tri.c), (tri.c, tri.a)];
        for (ia, ib) in edges {
            if let (Some((x0, y0)), Some((x1, y1))) = (projected[ia], projected[ib]) {
                bresenham(x0, y0, x1, y1, &mut set_pixel);
            }
        }
    }

    pixels
}

/// Bresenham's line algorithm; calls `plot` for each pixel on the line.
fn bresenham<F: FnMut(i32, i32)>(x0: i32, y0: i32, x1: i32, y1: i32, mut plot: F) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let (mut x, mut y) = (x0, y0);
    loop {
        plot(x, y);
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_has_8_vertices() {
        let m = Mesh::cube();
        assert_eq!(m.vertices.len(), 8);
    }

    #[test]
    fn icosahedron_has_correct_counts() {
        let m = Mesh::icosahedron();
        assert_eq!(m.vertices.len(), 12);
        assert_eq!(m.triangles.len(), 20);
    }

    #[test]
    fn geodesic_sphere_subdivisions_increase_vertex_count() {
        let m0 = Mesh::geodesic_sphere(0);
        let m1 = Mesh::geodesic_sphere(1);
        let m2 = Mesh::geodesic_sphere(2);
        assert!(m1.vertices.len() > m0.vertices.len());
        assert!(m2.vertices.len() > m1.vertices.len());
    }

    #[test]
    fn orthographic_projection_correct() {
        // A vertex at (0,0,0) should project to screen centre.
        let v = Vec3::new(0.0, 0.0, 0.0);
        let (px, py) = project_orthographic(&v, 800, 600, 100.0).unwrap();
        assert_eq!(px, 400);
        assert_eq!(py, 300);
    }

    #[test]
    fn perspective_projection_returns_none_behind_camera() {
        let camera = Camera::new(
            Vec3::new(0.0, 0.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            60.0,
        );
        // Vertex behind the camera.
        let v = Vec3::new(0.0, 0.0, 10.0);
        let result = project_perspective(&v, &camera, 800, 600);
        assert!(result.is_none(), "vertex behind camera should be None");
    }

    #[test]
    fn rasterize_wireframe_correct_buffer_size() {
        let mesh = Mesh::cube().transform(0.3, 1.0);
        let camera = Camera::new(
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(0.0, 0.0, 0.0),
            60.0,
        );
        let buf = rasterize_wireframe(&mesh, &camera, 64, 48, (255, 255, 255));
        assert_eq!(buf.len(), 64 * 48 * 3);
    }
}
