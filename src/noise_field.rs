//! Perlin-style noise with domain warping and multi-octave FBM.

/// Build a 512-entry permutation table using Fisher-Yates shuffle with LCG.
pub fn permutation_table(seed: u64) -> [u8; 512] {
    let mut perm: [u8; 256] = [0u8; 256];
    for (i, p) in perm.iter_mut().enumerate() {
        *p = i as u8;
    }

    // LCG shuffle
    let mut state = seed.wrapping_add(1);
    let lcg = |s: u64| -> u64 {
        s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
    };

    for i in (1..256).rev() {
        state = lcg(state);
        let j = (state >> 33) as usize % (i + 1);
        perm.swap(i, j);
    }

    let mut table = [0u8; 512];
    for i in 0..256 {
        table[i] = perm[i];
        table[i + 256] = perm[i];
    }
    table
}

/// Perlin fade function: 6t^5 - 15t^4 + 10t^3.
#[inline]
pub fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation.
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Gradient in 2D: 8 gradient directions based on hash.
#[inline]
pub fn grad2d(hash: u8, x: f64, y: f64) -> f64 {
    match hash & 7 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        7 => -y,
        _ => 0.0,
    }
}

/// Standard 2D Perlin noise returning value in roughly [-1, 1].
pub fn perlin2d(x: f64, y: f64, perm: &[u8; 512]) -> f64 {
    let xi = x.floor() as i64;
    let yi = y.floor() as i64;

    let xf = x - xi as f64;
    let yf = y - yi as f64;

    let u = fade(xf);
    let v = fade(yf);

    let xi = (xi & 255) as usize;
    let yi = (yi & 255) as usize;

    let aa = perm[perm[xi] as usize + yi] as usize;
    let ab = perm[perm[xi] as usize + yi + 1] as usize;
    let ba = perm[perm[xi + 1] as usize + yi] as usize;
    let bb = perm[perm[xi + 1] as usize + yi + 1] as usize;

    let g_aa = grad2d(perm[aa] as u8, xf, yf);
    let g_ba = grad2d(perm[ba] as u8, xf - 1.0, yf);
    let g_ab = grad2d(perm[ab] as u8, xf, yf - 1.0);
    let g_bb = grad2d(perm[bb] as u8, xf - 1.0, yf - 1.0);

    let x1 = lerp(g_aa, g_ba, u);
    let x2 = lerp(g_ab, g_bb, u);
    lerp(x1, x2, v)
}

/// Fractal Brownian Motion over Perlin noise, normalized output.
pub fn fbm2d(
    x: f64,
    y: f64,
    perm: &[u8; 512],
    octaves: u32,
    persistence: f64,
    lacunarity: f64,
) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut max_amplitude = 0.0;

    for _ in 0..octaves {
        value += perlin2d(x * frequency, y * frequency, perm) * amplitude;
        max_amplitude += amplitude;
        amplitude *= persistence;
        frequency *= lacunarity;
    }

    if max_amplitude > 0.0 {
        value / max_amplitude
    } else {
        0.0
    }
}

/// Noise field with configurable parameters.
pub struct NoiseField {
    pub perm: [u8; 512],
    pub scale: f64,
    pub octaves: u32,
    pub persistence: f64,
    pub lacunarity: f64,
}

impl NoiseField {
    pub fn new(seed: u64, scale: f64, octaves: u32) -> Self {
        Self {
            perm: permutation_table(seed),
            scale,
            octaves,
            persistence: 0.5,
            lacunarity: 2.0,
        }
    }

    /// Sample FBM noise at (x, y).
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        fbm2d(
            x * self.scale,
            y * self.scale,
            &self.perm,
            self.octaves,
            self.persistence,
            self.lacunarity,
        )
    }

    /// Domain-warped noise: warp coordinates before sampling.
    pub fn domain_warp(&self, x: f64, y: f64, warp_strength: f64) -> f64 {
        let wx = fbm2d(
            (x + 1.7) * self.scale,
            (y + 9.2) * self.scale,
            &self.perm,
            self.octaves,
            self.persistence,
            self.lacunarity,
        );
        let wy = fbm2d(
            (x + 8.3) * self.scale,
            (y + 2.8) * self.scale,
            &self.perm,
            self.octaves,
            self.persistence,
            self.lacunarity,
        );
        let xw = x + warp_strength * wx;
        let yw = y + warp_strength * wy;
        self.sample(xw, yw)
    }
}

/// Render grayscale noise image (width * height bytes, 0-255).
pub fn render_noise(field: &NoiseField, width: u32, height: u32, warp: f64) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for py in 0..height {
        for px in 0..width {
            let x = px as f64 / width as f64;
            let y = py as f64 / height as f64;
            let v = if warp.abs() > 1e-9 {
                field.domain_warp(x, y, warp)
            } else {
                field.sample(x, y)
            };
            // Map [-1, 1] to [0, 255]
            let byte = ((v + 1.0) * 0.5 * 255.0).clamp(0.0, 255.0) as u8;
            pixels.push(byte);
        }
    }
    pixels
}

/// Render RGB colored noise using a palette.
pub fn render_colored_noise(
    field: &NoiseField,
    width: u32,
    height: u32,
    palette: &[[u8; 3]],
) -> Vec<u8> {
    if palette.is_empty() {
        return vec![0u8; (width * height * 3) as usize];
    }
    let mut pixels = Vec::with_capacity((width * height * 3) as usize);
    for py in 0..height {
        for px in 0..width {
            let x = px as f64 / width as f64;
            let y = py as f64 / height as f64;
            let v = field.sample(x, y);
            // Map [-1, 1] to [0, 1]
            let t = ((v + 1.0) * 0.5).clamp(0.0, 1.0);
            let idx_f = t * (palette.len() - 1) as f64;
            let i0 = idx_f.floor() as usize;
            let i1 = (i0 + 1).min(palette.len() - 1);
            let frac = idx_f - i0 as f64;
            let c0 = palette[i0];
            let c1 = palette[i1];
            let r = (c0[0] as f64 + frac * (c1[0] as f64 - c0[0] as f64)) as u8;
            let g = (c0[1] as f64 + frac * (c1[1] as f64 - c0[1] as f64)) as u8;
            let b = (c0[2] as f64 + frac * (c1[2] as f64 - c0[2] as f64)) as u8;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perlin2d_in_range() {
        let perm = permutation_table(42);
        for i in 0..100 {
            let x = i as f64 * 0.1;
            let y = i as f64 * 0.07;
            let v = perlin2d(x, y, &perm);
            assert!(v >= -1.5 && v <= 1.5, "perlin2d out of expected range: {v}");
        }
    }

    #[test]
    fn test_fbm2d_in_reasonable_range() {
        let perm = permutation_table(99);
        for i in 0..50 {
            let x = i as f64 * 0.13;
            let y = i as f64 * 0.09;
            let v = fbm2d(x, y, &perm, 4, 0.5, 2.0);
            assert!(v >= -2.0 && v <= 2.0, "fbm2d out of range: {v}");
        }
    }

    #[test]
    fn test_domain_warp_differs_from_unwrapped() {
        let field = NoiseField::new(7, 1.0, 4);
        let x = 0.5;
        let y = 0.5;
        let plain = field.sample(x, y);
        let warped = field.domain_warp(x, y, 1.0);
        // They should differ (warp changes the coordinates)
        assert!((plain - warped).abs() > 1e-9, "domain warp should differ from plain sample");
    }

    #[test]
    fn test_render_returns_correct_size() {
        let field = NoiseField::new(1, 2.0, 3);
        let pixels = render_noise(&field, 32, 32, 0.0);
        assert_eq!(pixels.len(), 32 * 32);
        let colored = render_colored_noise(
            &field,
            16,
            16,
            &[[0, 0, 0], [128, 128, 128], [255, 255, 255]],
        );
        assert_eq!(colored.len(), 16 * 16 * 3);
    }

    #[test]
    fn test_permutation_table_has_256_unique_values_in_first_half() {
        let table = permutation_table(12345);
        let first_half = &table[..256];
        let mut seen = [false; 256];
        for &v in first_half {
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "first half should contain all 256 values");
    }
}
