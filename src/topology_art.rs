use std::f64::consts::PI;

/// Möbius strip parametric surface
/// u in [0, 2π], v in [-1, 1]
pub fn mobius_point(u: f64, v: f64) -> (f64, f64, f64) {
    let x = (1.0 + v/2.0 * (u/2.0).cos()) * u.cos();
    let y = (1.0 + v/2.0 * (u/2.0).cos()) * u.sin();
    let z = v/2.0 * (u/2.0).sin();
    (x, y, z)
}

/// Project 3D point onto 2D screen
pub fn project(x: f64, y: f64, z: f64, fov: f64, cx: f64, cy: f64) -> Option<(i32, i32)> {
    let d = z + 3.5; // distance from viewer
    if d < 0.01 { return None; }
    let px = (x * fov / d + cx) as i32;
    let py = (y * fov / d + cy) as i32;
    Some((px, py))
}

/// Renderer for topological surfaces.
pub struct TopologyRenderer {
    /// Output image width in pixels.
    pub width: u32,
    /// Output image height in pixels.
    pub height: u32,
}

impl TopologyRenderer {
    /// Create a new renderer for a canvas of the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        TopologyRenderer { width, height }
    }

    /// Render a Möbius strip rotated by `rotation` radians around the Y-axis.
    ///
    /// Returns RGBA pixel data (width × height × 4 bytes).
    pub fn render_mobius(&self, rotation: f64, steps_u: usize, steps_v: usize) -> Vec<u8> {
        let mut pixels = vec![20u8; (self.width * self.height * 4) as usize];
        // set alpha
        for i in 0..self.width*self.height { pixels[i as usize * 4 + 3] = 255; }

        let cx = self.width as f64 / 2.0;
        let cy = self.height as f64 / 2.0;
        let fov = self.width as f64 * 0.4;

        for ui in 0..steps_u {
            for vi in 0..steps_v {
                let u = ui as f64 / steps_u as f64 * 2.0 * PI;
                let v = vi as f64 / steps_v as f64 * 2.0 - 1.0;
                let (x, y, z) = mobius_point(u, v);
                // rotate around Y axis
                let rx = x * rotation.cos() - z * rotation.sin();
                let rz = x * rotation.sin() + z * rotation.cos();
                if let Some((px, py)) = project(rx, y, rz, fov, cx, cy) {
                    if px >= 0 && py >= 0 && (px as u32) < self.width && (py as u32) < self.height {
                        let pi = ((py as u32 * self.width + px as u32) * 4) as usize;
                        let hue = u / (2.0 * PI);
                        let r = (hue * 255.0) as u8;
                        let g = ((1.0 - hue) * 200.0 + 50.0) as u8;
                        let b = 200u8;
                        pixels[pi] = r; pixels[pi+1] = g; pixels[pi+2] = b;
                    }
                }
            }
        }
        pixels
    }

    /// Figure-8 Klein bottle (immersion in 3D)
    pub fn render_klein_bottle(&self, rotation: f64) -> Vec<u8> {
        let mut pixels = vec![10u8; (self.width * self.height * 4) as usize];
        for i in 0..self.width*self.height { pixels[i as usize * 4 + 3] = 255; }
        let cx = self.width as f64 / 2.0;
        let cy = self.height as f64 / 2.0;
        let fov = self.width as f64 * 0.35;
        let steps = 300;
        for ui in 0..steps {
            for vi in 0..steps {
                let u = ui as f64 / steps as f64 * 2.0 * PI;
                let v = vi as f64 / steps as f64 * 2.0 * PI;
                // Figure-8 Klein bottle parametrization
                let (x, y, z) = klein_bottle_point(u, v);
                let rx = x * rotation.cos() - z * rotation.sin();
                let rz = x * rotation.sin() + z * rotation.cos();
                if let Some((px, py)) = project(rx, y, rz, fov, cx, cy) {
                    if px >= 0 && py >= 0 && (px as u32) < self.width && (py as u32) < self.height {
                        let pi = ((py as u32 * self.width + px as u32) * 4) as usize;
                        let t = v / (2.0 * PI);
                        pixels[pi] = (50.0 + t * 180.0) as u8;
                        pixels[pi+1] = (100.0 + (1.0-t) * 155.0) as u8;
                        pixels[pi+2] = 220u8;
                    }
                }
            }
        }
        pixels
    }
}

fn klein_bottle_point(u: f64, v: f64) -> (f64, f64, f64) {
    // Figure-8 Klein bottle
    let r = 4.0 * (1.0 - (u / 2.0).cos() / 2.0);
    if u < PI {
        let x = 6.0 * u.cos() * (1.0 + u.sin()) + r * (u / 2.0).cos() * v.cos();
        let y = 16.0 * u.sin() + r * (u / 2.0).sin() * v.cos();
        let z = r * v.sin();
        (x / 10.0, y / 10.0, z / 10.0)
    } else {
        let x = 6.0 * u.cos() * (1.0 + u.sin()) + r * -(u / 2.0).cos() * v.cos();
        let y = 16.0 * u.sin() + r * (u / 2.0).sin() * v.cos();
        let z = r * v.sin();
        (x / 10.0, y / 10.0, z / 10.0)
    }
}
