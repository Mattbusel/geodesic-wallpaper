//! Cellular (Worley) noise: F1, F2 distance functions for procedural textures

/// Simple LCG hash for point generation
fn hash2(x: i32, y: i32) -> (f64, f64) {
    let mut h = (x as u64)
        .wrapping_mul(1664525)
        .wrapping_add(y as u64 * 22695477)
        .wrapping_add(1013904223);
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    let px = (h & 0xFFFF) as f64 / 65535.0;
    h = h.wrapping_mul(1664525).wrapping_add(1013904223);
    let py = (h & 0xFFFF) as f64 / 65535.0;
    (px, py)
}

/// How distance to a feature point is measured.
pub enum DistanceMetric {
    /// Straight-line distance, sqrt(dx^2 + dy^2): round cells.
    Euclidean,
    /// |dx| + |dy|: diamond-shaped cells.
    Manhattan,
    /// max(|dx|, |dy|): square cells.
    Chebyshev,
    /// (|dx|^p + |dy|^p)^(1/p) with the given exponent p.
    Minkowski(f64), // exponent p
}

impl DistanceMetric {
    /// Distance for the offset `(dx, dy)` under this metric.
    pub fn distance(&self, dx: f64, dy: f64) -> f64 {
        match self {
            DistanceMetric::Euclidean => (dx * dx + dy * dy).sqrt(),
            DistanceMetric::Manhattan => dx.abs() + dy.abs(),
            DistanceMetric::Chebyshev => dx.abs().max(dy.abs()),
            DistanceMetric::Minkowski(p) => (dx.abs().powf(*p) + dy.abs().powf(*p)).powf(1.0 / *p),
        }
    }
}

/// Compute F1 and F2 distances (distance to nearest and second-nearest feature point)
pub fn worley_f1_f2(x: f64, y: f64, metric: &DistanceMetric) -> (f64, f64) {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = x - ix as f64;
    let fy = y - iy as f64;

    let mut f1 = f64::MAX;
    let mut f2 = f64::MAX;

    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            let (px, py) = hash2(ix + dx, iy + dy);
            let ddx = dx as f64 + px - fx;
            let ddy = dy as f64 + py - fy;
            let d = metric.distance(ddx, ddy);
            if d < f1 {
                f2 = f1;
                f1 = d;
            } else if d < f2 {
                f2 = d;
            }
        }
    }
    (f1, f2)
}

/// Which combination of the nearest (F1) and second-nearest (F2) distances to output.
pub enum CellularMode {
    /// Distance to the nearest feature point.
    F1,
    /// Distance to the second-nearest feature point.
    F2,
    /// F2 - F1 (capped at 1): bright cell borders.
    F2MinusF1, // Cell borders
    /// Mean of F1 and F2.
    F1PlusF2,
    /// F1 * F2 (capped at 1).
    F1TimesF2,
}

/// Single-octave Worley noise at `(x, y)` (one feature point per unit cell).
pub fn cellular_noise(x: f64, y: f64, mode: &CellularMode, metric: &DistanceMetric) -> f64 {
    let (f1, f2) = worley_f1_f2(x, y, metric);
    match mode {
        CellularMode::F1 => f1,
        CellularMode::F2 => f2,
        CellularMode::F2MinusF1 => (f2 - f1).min(1.0),
        CellularMode::F1PlusF2 => (f1 + f2) * 0.5,
        CellularMode::F1TimesF2 => (f1 * f2).min(1.0),
    }
}

/// Multi-octave cellular noise (fractal Worley)
pub fn fbm_cellular(
    x: f64,
    y: f64,
    octaves: u32,
    lacunarity: f64,
    gain: f64,
    mode: &CellularMode,
    metric: &DistanceMetric,
) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_val = 0.0;
    for _ in 0..octaves {
        value += cellular_noise(x * frequency, y * frequency, mode, metric) * amplitude;
        max_val += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }
    value / max_val
}

/// Renders fractal cellular noise into an RGBA pixel buffer.
pub struct CellularNoiseRenderer {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Noise-space units spanned by the image (higher means more, smaller cells).
    pub scale: f64,
    /// Number of fractal octaves summed per pixel.
    pub octaves: u32,
}

impl CellularNoiseRenderer {
    /// Create a renderer of `width` x `height` pixels at the given scale, with 4 octaves.
    pub fn new(width: u32, height: u32, scale: f64) -> Self {
        CellularNoiseRenderer {
            width,
            height,
            scale,
            octaves: 4,
        }
    }

    /// Render to RGBA bytes (`width * height * 4`), colouring each noise value with `palette_fn`.
    pub fn render(
        &self,
        mode: &CellularMode,
        metric: &DistanceMetric,
        palette_fn: &dyn Fn(f64) -> (u8, u8, u8),
    ) -> Vec<u8> {
        let mut pixels = vec![0u8; (self.width * self.height * 4) as usize];
        for py in 0..self.height {
            for px in 0..self.width {
                let x = px as f64 / self.width as f64 * self.scale;
                let y = py as f64 / self.height as f64 * self.scale;
                let v = fbm_cellular(x, y, self.octaves, 2.0, 0.5, mode, metric);
                let (r, g, b) = palette_fn(v);
                let pi = ((py * self.width + px) * 4) as usize;
                pixels[pi] = r;
                pixels[pi + 1] = g;
                pixels[pi + 2] = b;
                pixels[pi + 3] = 255;
            }
        }
        pixels
    }
}
