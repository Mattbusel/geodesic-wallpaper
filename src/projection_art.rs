//! Non-Euclidean projections: hyperbolic disk and inversive geometry.
//!
//! Implements the Poincaré disk model of hyperbolic geometry, Möbius
//! transforms, circle inversions, Apollonian gaskets, and stereographic
//! projection.

// ---------------------------------------------------------------------------
// HyperbolicModel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperbolicModel {
    Poincare,
    Beltrami,
    Halfplane,
}

// ---------------------------------------------------------------------------
// HyperbolicPoint
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct HyperbolicPoint {
    pub x: f64,
    pub y: f64,
}

impl HyperbolicPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Norm squared: x^2 + y^2
    pub fn norm_sq(&self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    /// Check if point is inside the Poincaré unit disk.
    pub fn is_in_disk(&self) -> bool {
        self.norm_sq() < 1.0
    }
}

// ---------------------------------------------------------------------------
// hyperbolic_distance
// ---------------------------------------------------------------------------

/// Compute the hyperbolic distance between two Poincaré disk points.
pub fn hyperbolic_distance(a: &HyperbolicPoint, b: &HyperbolicPoint) -> f64 {
    // |a - b|^2
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let ab_sq = dx * dx + dy * dy;

    // |1 - conj(a)*b|^2 = 1 - 2(ax*bx + ay*by) + (ax^2+ay^2)*(bx^2+by^2)
    let dot = a.x * b.x + a.y * b.y;
    let denom_sq = 1.0 - 2.0 * dot + a.norm_sq() * b.norm_sq();

    if denom_sq <= 1e-12 || ab_sq < 0.0 {
        return 0.0;
    }

    let ratio = (ab_sq / denom_sq).sqrt();
    2.0 * ratio.atanh()
}

// ---------------------------------------------------------------------------
// Möbius transform (complex arithmetic)
// ---------------------------------------------------------------------------

/// Complex multiply: (a.re, a.im) * (b.re, b.im)
fn cmul(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Complex add
fn cadd(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    (a.0 + b.0, a.1 + b.1)
}

/// Complex divide
fn cdiv(num: (f64, f64), den: (f64, f64)) -> (f64, f64) {
    let denom = den.0 * den.0 + den.1 * den.1;
    if denom < 1e-15 {
        return (f64::INFINITY, f64::INFINITY);
    }
    (
        (num.0 * den.0 + num.1 * den.1) / denom,
        (num.1 * den.0 - num.0 * den.1) / denom,
    )
}

/// Complex conjugate
fn conj(z: (f64, f64)) -> (f64, f64) {
    (z.0, -z.1)
}

/// Möbius transform: f(z) = (a*z + b) / (conj(b)*z + conj(a))
/// This is a unit disk isometry when |a|^2 - |b|^2 = 1.
pub fn mobius_transform(z: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let num = cadd(cmul(a, z), b);
    let den = cadd(cmul(conj(b), z), conj(a));
    cdiv(num, den)
}

// ---------------------------------------------------------------------------
// Geodesic interpolation
// ---------------------------------------------------------------------------

/// Interpolate between two hyperbolic points using a Möbius transform.
/// Maps a → 0, walks along the geodesic, maps back.
pub fn hyperbolic_geodesic(
    a: &HyperbolicPoint,
    b: &HyperbolicPoint,
    steps: usize,
) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(steps + 1);

    // Map a to origin: T(z) = (z - a) / (1 - conj(a)*z)
    let za = (a.x, a.y);
    let zb = (b.x, b.y);

    // Transform b through the map that sends a → 0
    // T(z) = (z - a) / (1 - conj(a)*z)
    let map_to_origin = |z: (f64, f64)| -> (f64, f64) {
        let num = (z.0 - za.0, z.1 - za.1);
        let den = cadd((1.0, 0.0), cmul((-conj(za).0, -conj(za).1), z));
        // den = 1 - conj(a)*z
        let den2 = cadd((1.0, 0.0), cmul(conj(za), z));
        // Actually: 1 - conj(a)*z = (1.0,0) - conj(za)*z
        let _ = den; // suppress warning
        let den_final = (1.0 - conj(za).0 * z.0 + conj(za).1 * z.1,
                         -(conj(za).0 * z.1 + conj(za).1 * z.0));
        let _ = den2;
        cdiv(num, den_final)
    };

    let map_from_origin = |z: (f64, f64)| -> (f64, f64) {
        // Inverse: T^{-1}(w) = (w + a) / (1 + conj(a)*w)
        let num = cadd(z, za);
        let den = cadd((1.0, 0.0), cmul(conj(za), z));
        cdiv(num, den)
    };

    let b_mapped = map_to_origin(zb);

    // In the origin-centered frame, geodesic from 0 to b_mapped is a straight line
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let w = (b_mapped.0 * t, b_mapped.1 * t);
        let p = map_from_origin(w);
        points.push(p);
    }

    points
}

// ---------------------------------------------------------------------------
// Screen mapping
// ---------------------------------------------------------------------------

/// Map a Poincaré disk point (range [-1, 1]^2) to screen coordinates.
pub fn poincare_to_screen(p: &HyperbolicPoint, width: u32, height: u32) -> (u32, u32) {
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let radius = cx.min(cy) * 0.95;

    let sx = (cx + p.x * radius).clamp(0.0, width as f64 - 1.0) as u32;
    let sy = (cy + p.y * radius).clamp(0.0, height as f64 - 1.0) as u32;
    (sx, sy)
}

// ---------------------------------------------------------------------------
// Render Poincaré disk
// ---------------------------------------------------------------------------

/// Render a Poincaré disk with random geodesics. Returns an RGB pixel buffer.
pub fn render_poincare_disk(
    width: u32,
    height: u32,
    num_geodesics: usize,
    seed: u64,
) -> Vec<u8> {
    let mut buf = vec![10u8; (width * height * 3) as usize];

    // Draw the boundary circle
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let radius = cx.min(cy) * 0.95;

    for i in 0..360 * 4 {
        let angle = i as f64 * std::f64::consts::TAU / (360.0 * 4.0);
        let px = (cx + radius * angle.cos()) as u32;
        let py = (cy + radius * angle.sin()) as u32;
        if px < width && py < height {
            let idx = ((py * width + px) * 3) as usize;
            buf[idx] = 180;
            buf[idx + 1] = 180;
            buf[idx + 2] = 180;
        }
    }

    // LCG random generator
    let mut state = seed;
    let mut lcg = move || -> u64 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        state
    };

    let rand_disk_point = |lcg: &mut dyn FnMut() -> u64| -> HyperbolicPoint {
        loop {
            // Points in [-0.9, 0.9]
            let x = (lcg() % 1000) as f64 / 1000.0 * 1.8 - 0.9;
            let y = (lcg() % 1000) as f64 / 1000.0 * 1.8 - 0.9;
            let p = HyperbolicPoint::new(x, y);
            if p.norm_sq() < 0.81 {
                return p;
            }
        }
    };

    // Color palette
    let colors: [[u8; 3]; 6] = [
        [255, 100, 100],
        [100, 255, 100],
        [100, 100, 255],
        [255, 220, 100],
        [220, 100, 255],
        [100, 255, 220],
    ];

    for g in 0..num_geodesics {
        let a = rand_disk_point(&mut lcg);
        let b = rand_disk_point(&mut lcg);
        let color = colors[g % colors.len()];

        let geo = hyperbolic_geodesic(&a, &b, 200);
        for pt in &geo {
            let hp = HyperbolicPoint::new(pt.0, pt.1);
            if hp.norm_sq() < 1.0 {
                let (sx, sy) = poincare_to_screen(&hp, width, height);
                if sx < width && sy < height {
                    let idx = ((sy * width + sx) * 3) as usize;
                    buf[idx]     = color[0];
                    buf[idx + 1] = color[1];
                    buf[idx + 2] = color[2];
                }
            }
        }
    }

    buf
}

// ---------------------------------------------------------------------------
// InversiveCircle and circle inversion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InversiveCircle {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
}

/// Invert point (px, py) through `circle`.
/// Returns (f64::INFINITY, f64::INFINITY) if point is the center.
pub fn circle_inversion(px: f64, py: f64, circle: &InversiveCircle) -> (f64, f64) {
    let dx = px - circle.cx;
    let dy = py - circle.cy;
    let dist_sq = dx * dx + dy * dy;
    if dist_sq < 1e-15 {
        return (f64::INFINITY, f64::INFINITY);
    }
    let factor = circle.r * circle.r / dist_sq;
    (circle.cx + dx * factor, circle.cy + dy * factor)
}

// ---------------------------------------------------------------------------
// Apollonian gasket
// ---------------------------------------------------------------------------

/// Recursively generate an Apollonian gasket.
/// Places 3 inner circles at 120° angles, each with radius `r/3`, and recurses.
pub fn apollonian_gasket(cx: f64, cy: f64, r: f64, depth: usize) -> Vec<InversiveCircle> {
    let mut circles = Vec::new();

    if depth == 0 || r < 0.5 {
        return circles;
    }

    let inner_r = r / 3.0;
    let dist = r * (2.0 / 3.0);

    for k in 0..3 {
        let angle = k as f64 * std::f64::consts::TAU / 3.0;
        let icx = cx + dist * angle.cos();
        let icy = cy + dist * angle.sin();
        circles.push(InversiveCircle { cx: icx, cy: icy, r: inner_r });
        let children = apollonian_gasket(icx, icy, inner_r, depth - 1);
        circles.extend(children);
    }

    circles
}

// ---------------------------------------------------------------------------
// Render Apollonian gasket
// ---------------------------------------------------------------------------

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
    let h = h % 1.0;
    let i = (h * 6.0) as u32;
    let f = h * 6.0 - i as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);

    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Render Apollonian gasket circles as outlines on an RGB buffer.
pub fn render_apollonian(circles: &[InversiveCircle], width: u32, height: u32) -> Vec<u8> {
    let mut buf = vec![20u8; (width * height * 3) as usize];

    let n = circles.len().max(1);
    for (i, circle) in circles.iter().enumerate() {
        let hue = i as f64 / n as f64;
        let color = hsv_to_rgb(hue, 0.9, 0.95);

        // Draw circle outline by sampling angles
        let circumference = 2.0 * std::f64::consts::PI * circle.r;
        let steps = (circumference * 2.0) as usize + 16;

        for s in 0..steps {
            let angle = s as f64 * std::f64::consts::TAU / steps as f64;
            let px = circle.cx + circle.r * angle.cos();
            let py = circle.cy + circle.r * angle.sin();

            if px >= 0.0 && px < width as f64 && py >= 0.0 && py < height as f64 {
                let ix = px as u32;
                let iy = py as u32;
                let idx = ((iy * width + ix) * 3) as usize;
                buf[idx]     = color[0];
                buf[idx + 1] = color[1];
                buf[idx + 2] = color[2];
            }
        }
    }

    buf
}

// ---------------------------------------------------------------------------
// Stereographic projection
// ---------------------------------------------------------------------------

/// Project sphere coordinates (theta, phi) to plane via stereographic projection
/// from the north pole. Returns (x, y) plane coordinates.
pub fn stereographic_to_plane(theta: f64, phi: f64) -> (f64, f64) {
    // Sphere point: (sin(theta)*cos(phi), sin(theta)*sin(phi), cos(theta))
    let x = theta.sin() * phi.cos();
    let y = theta.sin() * phi.sin();
    let z = theta.cos();

    // Project from north pole (0,0,1): P' = (x, y) / (1 - z)
    let denom = 1.0 - z;
    if denom.abs() < 1e-12 {
        // At the north pole — maps to infinity
        return (f64::INFINITY, f64::INFINITY);
    }
    (x / denom, y / denom)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperbolic_distance_at_origin() {
        let a = HyperbolicPoint::new(0.0, 0.0);
        let b = HyperbolicPoint::new(0.0, 0.0);
        let d = hyperbolic_distance(&a, &b);
        assert!(d.abs() < 1e-10, "Distance from origin to itself should be 0, got {}", d);
    }

    #[test]
    fn test_hyperbolic_distance_positive() {
        let a = HyperbolicPoint::new(0.0, 0.0);
        let b = HyperbolicPoint::new(0.5, 0.0);
        let d = hyperbolic_distance(&a, &b);
        assert!(d > 0.0, "Distance should be positive");
    }

    #[test]
    fn test_mobius_transform_preserves_unit_disk() {
        // Identity-like transform: a=1, b=0
        let z = (0.3, 0.4);
        let a = (1.0, 0.0);
        let b = (0.0, 0.0);
        let w = mobius_transform(z, a, b);
        // Should be close to z itself
        assert!((w.0 - z.0).abs() < 1e-10);
        assert!((w.1 - z.1).abs() < 1e-10);

        // Result should stay in or near unit disk for valid disk isometry
        let r_sq = w.0 * w.0 + w.1 * w.1;
        assert!(r_sq <= 1.1, "Result should be inside unit disk, r^2={}", r_sq);
    }

    #[test]
    fn test_poincare_to_screen_within_bounds() {
        let width = 800u32;
        let height = 600u32;
        let points = [
            HyperbolicPoint::new(0.0, 0.0),
            HyperbolicPoint::new(0.9, 0.0),
            HyperbolicPoint::new(-0.9, 0.0),
            HyperbolicPoint::new(0.0, 0.9),
            HyperbolicPoint::new(0.6, 0.6),
        ];
        for p in &points {
            let (sx, sy) = poincare_to_screen(p, width, height);
            assert!(sx < width, "sx={} out of bounds for p=({},{})", sx, p.x, p.y);
            assert!(sy < height, "sy={} out of bounds for p=({},{})", sy, p.x, p.y);
        }
    }

    #[test]
    fn test_circle_inversion_center_is_infinity() {
        let circle = InversiveCircle { cx: 0.0, cy: 0.0, r: 1.0 };
        let (px, py) = circle_inversion(0.0, 0.0, &circle);
        assert!(px.is_infinite() || py.is_infinite(), "Inversion of center should be infinity");
    }

    #[test]
    fn test_circle_inversion_on_circle() {
        // Point on the circle should map to itself
        let circle = InversiveCircle { cx: 0.0, cy: 0.0, r: 2.0 };
        let (px, py) = circle_inversion(2.0, 0.0, &circle);
        assert!((px - 2.0).abs() < 1e-10);
        assert!((py - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_apollonian_gasket_non_empty() {
        let circles = apollonian_gasket(400.0, 300.0, 200.0, 2);
        assert!(!circles.is_empty(), "Apollonian gasket should have circles");
    }

    #[test]
    fn test_render_poincare_disk_correct_size() {
        let buf = render_poincare_disk(100, 100, 3, 42);
        assert_eq!(buf.len(), 100 * 100 * 3);
    }

    #[test]
    fn test_render_apollonian_correct_size() {
        let circles = apollonian_gasket(50.0, 50.0, 40.0, 2);
        let buf = render_apollonian(&circles, 100, 100);
        assert_eq!(buf.len(), 100 * 100 * 3);
    }

    #[test]
    fn test_stereographic_south_pole() {
        // South pole: theta=pi → maps to (0, 0)
        let (x, y) = stereographic_to_plane(std::f64::consts::PI, 0.0);
        assert!(x.abs() < 1e-10, "South pole should map to x=0");
        assert!(y.abs() < 1e-10, "South pole should map to y=0");
    }
}
