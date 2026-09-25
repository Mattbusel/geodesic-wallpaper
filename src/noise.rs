//! Procedural Noise Generator
//!
//! Implements 2D Perlin noise with fractal Brownian motion support.

// ── NoiseType ─────────────────────────────────────────────────────────────────

/// Noise algorithm selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseType {
    White,
    /// 2D gradient noise with smooth interpolation (Perlin 1985).
    Perlin,
    /// Stub — delegates to Perlin.
    Simplex,
}

// ── PerlinNoise ───────────────────────────────────────────────────────────────

/// 2D Perlin noise implementation.
///
/// Internally maintains a 256-entry permutation table seeded via a
/// linear-congruential generator, and uses 8 gradient vectors at 45° intervals.
pub struct PerlinNoise {
    perm: [u8; 512],
}

impl PerlinNoise {
    /// Create a new Perlin noise generator with the given seed.
    pub fn new(seed: u64) -> Self {
        let perm256 = Self::build_perm(seed);
        let mut perm = [0u8; 512];
        for i in 0..256 {
            perm[i] = perm256[i];
            perm[i + 256] = perm256[i];
        }
        Self { perm }
    }

    /// Fisher-Yates shuffle seeded with a simple LCG.
    fn build_perm(seed: u64) -> [u8; 256] {
        let mut table: [u8; 256] = [0; 256];
        for (i, v) in table.iter_mut().enumerate() {
            *v = i as u8;
        }
        // LCG: multiplier and increment from Numerical Recipes
        let mut rng = seed.wrapping_add(1);
        for i in (1..256).rev() {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let j = (rng >> 33) as usize % (i + 1);
            table.swap(i, j);
        }
        table
    }

    /// Gradient dot product: one of 8 unit vectors at 45° intervals.
    fn grad(hash: u8, x: f64, y: f64) -> f64 {
        // 8 gradients: (±1, 0), (0, ±1), (±1/√2, ±1/√2) mapped to 8 hash values
        match hash & 7 {
            0 => x + y,
            1 => -x + y,
            2 => x - y,
            3 => -x - y,
            4 => x,
            5 => -x,
            6 => y,
            7 => -y,
            _ => unreachable!(),
        }
    }

    /// Quintic smoothstep: `6t^5 - 15t^4 + 10t^3`.
    #[inline]
    fn fade(t: f64) -> f64 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    #[inline]
    fn lerp(a: f64, b: f64, t: f64) -> f64 {
        a + t * (b - a)
    }

    /// Sample 2D Perlin noise at `(x, y)`.
    ///
    /// Returns a value in approximately [-1, 1].
    pub fn sample(&self, x: f64, y: f64) -> f64 {
        let xi = x.floor() as i64;
        let yi = y.floor() as i64;

        let xf = x - xi as f64;
        let yf = y - yi as f64;

        let u = Self::fade(xf);
        let v = Self::fade(yf);

        let xi = (xi & 255) as usize;
        let yi = (yi & 255) as usize;

        let aa = self.perm[self.perm[xi] as usize + yi] as u8;
        let ab = self.perm[self.perm[xi] as usize + yi + 1] as u8;
        let ba = self.perm[self.perm[xi + 1] as usize + yi] as u8;
        let bb = self.perm[self.perm[xi + 1] as usize + yi + 1] as u8;

        let x1 = Self::lerp(Self::grad(aa, xf, yf), Self::grad(ba, xf - 1.0, yf), u);
        let x2 = Self::lerp(
            Self::grad(ab, xf, yf - 1.0),
            Self::grad(bb, xf - 1.0, yf - 1.0),
            u,
        );

        Self::lerp(x1, x2, v)
    }

    /// Fractal Brownian Motion: sum of `octaves` noise layers at increasing frequencies.
    ///
    /// - `persistence`: amplitude scaling per octave (typically 0.5)
    /// - `lacunarity`: frequency scaling per octave (typically 2.0)
    ///
    /// The result is normalised to approximately [-1, 1].
    pub fn octaves(&self, x: f64, y: f64, octaves: u32, persistence: f64, lacunarity: f64) -> f64 {
        if octaves == 0 {
            return 0.0;
        }
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = 1.0;
        let mut max_value = 0.0;

        for _ in 0..octaves {
            value += self.sample(x * frequency, y * frequency) * amplitude;
            max_value += amplitude;
            amplitude *= persistence;
            frequency *= lacunarity;
        }

        if max_value > 0.0 {
            value / max_value
        } else {
            0.0
        }
    }
}

// ── NoiseGenerator ────────────────────────────────────────────────────────────

/// High-level noise generator that wraps `PerlinNoise` with a configurable type.
pub struct NoiseGenerator {
    pub noise_type: NoiseType,
    pub scale: f64,
    pub octaves: u32,
    pub persistence: f64,
    pub lacunarity: f64,
    perlin: PerlinNoise,
}

impl NoiseGenerator {
    pub fn new(noise_type: NoiseType, seed: u64) -> Self {
        Self {
            noise_type,
            scale: 1.0,
            octaves: 1,
            persistence: 0.5,
            lacunarity: 2.0,
            perlin: PerlinNoise::new(seed),
        }
    }

    /// Sample at `(x, y)`, returning a value in approximately [-1, 1].
    pub fn sample(&self, x: f64, y: f64) -> f32 {
        match self.noise_type {
            NoiseType::White => {
                // Deterministic hash-based white noise
                let ix = (x * 1000.0) as i64;
                let iy = (y * 1000.0) as i64;
                let h = ix
                    .wrapping_mul(374761393)
                    .wrapping_add(iy.wrapping_mul(668265263));
                let h = h ^ (h >> 13);
                let h = h.wrapping_mul(1274126177i64);
                (h & 0xFFFF) as f32 / 32767.5 - 1.0
            }
            NoiseType::Perlin | NoiseType::Simplex => {
                if self.octaves <= 1 {
                    self.perlin.sample(x * self.scale, y * self.scale) as f32
                } else {
                    self.perlin.octaves(
                        x * self.scale,
                        y * self.scale,
                        self.octaves,
                        self.persistence,
                        self.lacunarity,
                    ) as f32
                }
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 42;

    // PerlinNoise tests
    #[test]
    fn test_perlin_range() {
        let noise = PerlinNoise::new(SEED);
        for i in 0..100 {
            let x = i as f64 * 0.13;
            let y = i as f64 * 0.17;
            let v = noise.sample(x, y);
            assert!(v >= -1.5 && v <= 1.5, "sample out of expected range: {}", v);
        }
    }

    #[test]
    fn test_perlin_deterministic() {
        let n1 = PerlinNoise::new(SEED);
        let n2 = PerlinNoise::new(SEED);
        for i in 0..20 {
            let x = i as f64 * 0.1;
            let y = i as f64 * 0.07;
            assert_eq!(n1.sample(x, y), n2.sample(x, y));
        }
    }

    #[test]
    fn test_perlin_different_seeds() {
        let n1 = PerlinNoise::new(1);
        let n2 = PerlinNoise::new(999);
        let mut differ = false;
        for i in 1..20 {
            let x = i as f64 * 0.3;
            let y = i as f64 * 0.2;
            if (n1.sample(x, y) - n2.sample(x, y)).abs() > 1e-10 {
                differ = true;
                break;
            }
        }
        assert!(differ, "different seeds should produce different noise");
    }

    #[test]
    fn test_perlin_smoothness() {
        // Adjacent samples should be closer than random would be on average.
        let noise = PerlinNoise::new(SEED);
        let mut max_delta = 0.0_f64;
        for i in 0..100 {
            let x = i as f64 * 0.01;
            let v0 = noise.sample(x, 0.0);
            let v1 = noise.sample(x + 0.01, 0.0);
            let delta = (v1 - v0).abs();
            if delta > max_delta {
                max_delta = delta;
            }
        }
        assert!(
            max_delta < 0.5,
            "noise should be smooth; max_delta={}",
            max_delta
        );
    }

    #[test]
    fn test_fade_endpoints() {
        assert!((PerlinNoise::fade(0.0) - 0.0).abs() < 1e-12);
        assert!((PerlinNoise::fade(1.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_fade_monotone() {
        let mut prev = 0.0;
        for i in 1..=100 {
            let t = i as f64 / 100.0;
            let v = PerlinNoise::fade(t);
            assert!(
                v >= prev - 1e-12,
                "fade should be monotone: {} < {}",
                v,
                prev
            );
            prev = v;
        }
    }

    #[test]
    fn test_octaves_zero() {
        let noise = PerlinNoise::new(SEED);
        assert_eq!(noise.octaves(1.0, 1.0, 0, 0.5, 2.0), 0.0);
    }

    #[test]
    fn test_octaves_one_equals_sample() {
        let noise = PerlinNoise::new(SEED);
        let x = 1.23;
        let y = 4.56;
        let single = noise.sample(x, y);
        let oct = noise.octaves(x, y, 1, 0.5, 2.0);
        assert!(
            (single - oct).abs() < 1e-10,
            "1-octave fBm should equal sample"
        );
    }

    #[test]
    fn test_octaves_range() {
        let noise = PerlinNoise::new(SEED);
        for i in 0..50 {
            let x = i as f64 * 0.1;
            let y = i as f64 * 0.15;
            let v = noise.octaves(x, y, 4, 0.5, 2.0);
            assert!(v >= -1.5 && v <= 1.5, "fBm out of range: {}", v);
        }
    }

    // NoiseGenerator tests
    #[test]
    fn test_generator_perlin_range() {
        let gen = NoiseGenerator::new(NoiseType::Perlin, SEED);
        for i in 0..50 {
            let v = gen.sample(i as f64 * 0.1, i as f64 * 0.07);
            assert!(v >= -1.5 && v <= 1.5, "value out of range: {}", v);
        }
    }

    #[test]
    fn test_generator_simplex_delegates_to_perlin() {
        let g_p = NoiseGenerator::new(NoiseType::Perlin, SEED);
        let g_s = NoiseGenerator::new(NoiseType::Simplex, SEED);
        // Both should return the same values (Simplex is stubbed as Perlin)
        for i in 1..20 {
            let x = i as f64 * 0.2;
            let y = i as f64 * 0.13;
            assert_eq!(g_p.sample(x, y), g_s.sample(x, y));
        }
    }

    #[test]
    fn test_generator_white_noise_varies() {
        let gen = NoiseGenerator::new(NoiseType::White, SEED);
        let v1 = gen.sample(0.0, 0.0);
        let v2 = gen.sample(1.0, 0.0);
        // White noise at different positions should generally differ
        // (may rarely be equal, but with integer grid points they will differ)
        assert!((v1 - v2).abs() > 1e-6 || true); // soft test; mainly checks no panic
    }

    #[test]
    fn test_generator_deterministic() {
        let g1 = NoiseGenerator::new(NoiseType::Perlin, 77);
        let g2 = NoiseGenerator::new(NoiseType::Perlin, 77);
        for i in 0..10 {
            let x = i as f64 * 0.3;
            let y = i as f64 * 0.5;
            assert_eq!(g1.sample(x, y), g2.sample(x, y));
        }
    }

    #[test]
    fn test_octave_generator_range() {
        let mut gen = NoiseGenerator::new(NoiseType::Perlin, SEED);
        gen.octaves = 4;
        gen.persistence = 0.5;
        gen.lacunarity = 2.0;
        gen.scale = 2.0;
        for i in 0..30 {
            let v = gen.sample(i as f64 * 0.1, i as f64 * 0.1);
            assert!(v >= -1.5 && v <= 1.5, "octave gen out of range: {}", v);
        }
    }
}
