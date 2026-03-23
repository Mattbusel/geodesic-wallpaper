use std::f64::consts::PI;

/// Complex number operations for Möbius transforms
fn cmul(a: (f64,f64), b: (f64,f64)) -> (f64,f64) { (a.0*b.0-a.1*b.1, a.0*b.1+a.1*b.0) }
fn cdiv(a: (f64,f64), b: (f64,f64)) -> (f64,f64) {
    let d = b.0*b.0+b.1*b.1;
    ((a.0*b.0+a.1*b.1)/d, (a.1*b.0-a.0*b.1)/d)
}
fn conj(a: (f64,f64)) -> (f64,f64) { (a.0, -a.1) }
fn cnorm(a: (f64,f64)) -> f64 { (a.0*a.0+a.1*a.1).sqrt() }

/// Poincaré disk: check if point is inside
pub fn in_disk(z: (f64,f64)) -> bool { z.0*z.0 + z.1*z.1 < 1.0 }

/// Hyperbolic rotation by angle theta
pub fn hyperbolic_rotate(z: (f64,f64), theta: f64) -> (f64,f64) {
    cmul(z, (theta.cos(), theta.sin()))
}

/// Möbius transform: move point p to origin
pub fn mobius_to_origin(z: (f64,f64), p: (f64,f64)) -> (f64,f64) {
    // (z - p) / (1 - conj(p)*z)
    let num = (z.0 - p.0, z.1 - p.1);
    let den = {
        let cp = conj(p);
        let cpz = cmul(cp, z);
        (1.0 - cpz.0, -cpz.1)
    };
    cdiv(num, den)
}

/// A regular {p, q} hyperbolic tessellation in the Poincaré disk.
pub struct HyperbolicTiling {
    /// Number of sides of each polygon.
    pub p: u32,
    /// Number of polygons meeting at each vertex.
    pub q: u32,
}

impl HyperbolicTiling {
    /// Create a new tiling. Panics unless (p-2)(q-2) > 4 (hyperbolic condition).
    pub fn new(p: u32, q: u32) -> Self {
        assert!(p >= 3 && q >= 3 && (p-2)*(q-2) > 4, "Must be hyperbolic: (p-2)(q-2) > 4");
        HyperbolicTiling { p, q }
    }

    /// Circumradius of the fundamental polygon in the Poincaré disk.
    pub fn circumradius(&self) -> f64 {
        let p = self.p as f64;
        let q = self.q as f64;
        // r = tanh(acosh(cos(π/q) / sin(π/p)))
        let cos_pi_q = (PI/q).cos();
        let sin_pi_p = (PI/p).sin();
        let arg = cos_pi_q / sin_pi_p;
        let r_hyp = arg.acosh();
        r_hyp.tanh()
    }

    /// Vertices of the fundamental {p, q} polygon centered at the origin.
    pub fn polygon_vertices(&self) -> Vec<(f64,f64)> {
        let r = self.circumradius();
        (0..self.p).map(|k| {
            let angle = 2.0 * PI * k as f64 / self.p as f64;
            (r * angle.cos(), r * angle.sin())
        }).collect()
    }

    /// Render the hyperbolic tiling as an RGBA image.
    ///
    /// Colors each "cell" based on its depth from the tiling origin.
    pub fn render(&self, width: u32, height: u32, depth: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let r = self.circumradius();

        for py in 0..height {
            for px in 0..width {
                // map pixel to Poincaré disk [-1,1] x [-1,1]
                let dx = (px as f64 / width as f64 * 2.0 - 1.0) * 0.98;
                let dy = (py as f64 / height as f64 * 2.0 - 1.0) * 0.98;
                if dx*dx + dy*dy >= 0.98*0.98 {
                    // outside disk - dark background
                    let pi = ((py * width + px) * 4) as usize;
                    pixels[pi+3] = 255;
                    continue;
                }
                // Determine which tile this point belongs to by fundamental domain
                // Simple approach: color by angle of closest polygon center
                let mut z = (dx, dy);
                let mut level = 0u32;
                // Iterate Möbius transforms to normalize
                for _ in 0..depth {
                    let norm = cnorm(z);
                    if norm < r * 0.5 { break; }
                    // Find nearest polygon center among rotations
                    let mut best_dist = f64::MAX;
                    let mut best_center = (0.0, 0.0);
                    for k in 0..self.p {
                        let angle = 2.0 * PI * k as f64 / self.p as f64;
                        let center = (r * angle.cos(), r * angle.sin());
                        let d = {
                            let mz = mobius_to_origin(z, center);
                            cnorm(mz)
                        };
                        if d < best_dist {
                            best_dist = d;
                            best_center = center;
                        }
                    }
                    z = mobius_to_origin(z, best_center);
                    level += 1;
                }
                let angle = z.1.atan2(z.0);
                let t = (angle / (2.0 * PI) + 0.5).fract();
                let brightness = 1.0 - (level as f64 / depth as f64).min(1.0) * 0.5;
                let pi = ((py * width + px) * 4) as usize;
                pixels[pi] = ((0.5 + 0.5 * (t * 6.28).sin()) * brightness * 255.0) as u8;
                pixels[pi+1] = ((0.5 + 0.5 * (t * 6.28 + 2.09).sin()) * brightness * 255.0) as u8;
                pixels[pi+2] = ((0.5 + 0.5 * (t * 6.28 + 4.19).sin()) * brightness * 255.0) as u8;
                pixels[pi+3] = 255;
            }
        }
        pixels
    }
}
