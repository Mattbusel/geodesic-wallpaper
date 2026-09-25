//! Procedural texture synthesis.

use crate::generative_art::{fbm, lerp_color};
use std::f64::consts::PI;

/// Types of procedural textures that can be synthesized.
#[derive(Debug, Clone, PartialEq)]
pub enum TextureType {
    Wood,
    Marble,
    Brick,
    Fabric,
    Metal,
    Water,
    Clouds,
    Lava,
    Concrete,
    Sand,
}

/// Parameters controlling texture synthesis.
#[derive(Debug, Clone)]
pub struct TextureParams {
    pub texture_type: TextureType,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub scale: f64,
    pub color_a: [u8; 3],
    pub color_b: [u8; 3],
    pub variation: f64,
}

/// Synthesizes procedural textures as pixel grids.
pub struct TextureSynthesizer;

impl TextureSynthesizer {
    /// Dispatch to the appropriate synthesizer.
    pub fn synthesize(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        match params.texture_type {
            TextureType::Wood => Self::wood(params),
            TextureType::Marble => Self::marble(params),
            TextureType::Brick => Self::brick(params),
            TextureType::Fabric => Self::fabric(params),
            TextureType::Metal => Self::metal(params),
            TextureType::Water => Self::water(params),
            TextureType::Clouds => Self::clouds(params),
            TextureType::Lava => Self::lava(params),
            TextureType::Concrete => Self::concrete(params),
            TextureType::Sand => Self::sand(params),
        }
    }

    /// Wood: radial rings with noise distortion.
    pub fn wood(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let cx = w as f64 * 0.5;
        let cy = h as f64 * 0.5;
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = (x as f64 - cx) / w as f64 * params.scale;
                let ny = (y as f64 - cy) / h as f64 * params.scale;

                // Add noise distortion
                let distort = turb(nx, ny, params.seed, 4) * params.variation;
                let dist = (nx * nx + ny * ny).sqrt() + distort;

                // Rings: sine of radial distance
                let ring = ((dist * 20.0).sin() + 1.0) * 0.5;
                image[y][x] = lerp_color(params.color_a, params.color_b, ring);
            }
        }
        image
    }

    /// Marble: sine veins with turbulence.
    pub fn marble(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;

                let t = turb(nx, ny, params.seed, 5);
                let vein = ((nx * 4.0 + t * params.variation).sin() + 1.0) * 0.5;
                image[y][x] = lerp_color(params.color_a, params.color_b, vein);
            }
        }
        image
    }

    /// Brick: grid with mortar gaps and per-brick color variation.
    pub fn brick(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        let brick_w = (w as f64 / params.scale * 0.2).max(4.0) as usize;
        let brick_h = (h as f64 / params.scale * 0.1).max(2.0) as usize;
        let mortar = 2usize;

        for y in 0..h {
            for x in 0..w {
                let row = y / brick_h;
                // Offset every other row
                let col_x = if row % 2 == 0 { x } else { x + brick_w / 2 };
                let col = col_x / brick_w;

                // Mortar check
                let in_mortar_v = (y % brick_h) < mortar;
                let in_mortar_h = (col_x % brick_w) < mortar;

                if in_mortar_v || in_mortar_h {
                    // Mortar color: grey-ish blend
                    let mortar_color = lerp_color(params.color_a, [180, 180, 180], 0.8);
                    image[y][x] = mortar_color;
                } else {
                    // Brick with per-brick variation
                    let brick_hash = simple_hash(col as u64, row as u64, params.seed);
                    let variation = (brick_hash as f64 / u64::MAX as f64) * params.variation;
                    image[y][x] = lerp_color(params.color_a, params.color_b, variation);
                }
            }
        }
        image
    }

    /// Fabric: woven pattern with thread simulation.
    pub fn fabric(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];
        let freq = params.scale * 10.0;

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64;
                let ny = y as f64 / h as f64;

                // Woven: alternate thread direction
                let warp = ((nx * freq).sin()).abs(); // vertical threads
                let weft = ((ny * freq).sin()).abs(); // horizontal threads

                let t = if (x + y) % 2 == 0 {
                    warp * (1.0 - weft * 0.3)
                } else {
                    weft * (1.0 - warp * 0.3)
                };

                // Add slight noise for texture
                let noise = fbm(
                    nx * params.scale * 2.0,
                    ny * params.scale * 2.0,
                    2,
                    params.seed,
                ) * 0.05;
                let t = (t + noise).clamp(0.0, 1.0);
                image[y][x] = lerp_color(params.color_a, params.color_b, t);
            }
        }
        image
    }

    /// Metal: brushed directional noise with specular highlight.
    pub fn metal(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;

                // Directional brushed lines along x-axis
                let brush = fbm(nx * 0.1, ny * 3.0, 3, params.seed) * params.variation;

                // Specular highlight: bright stripe near center
                let spec_pos = 0.5f64;
                let spec = (-(ny / params.scale - spec_pos).powi(2) / 0.01).exp() * 0.4;

                let t = (brush + spec).clamp(0.0, 1.0);
                image[y][x] = lerp_color(params.color_a, params.color_b, t);
            }
        }
        image
    }

    /// Water: ripple interference pattern.
    pub fn water(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        // Two wave sources
        let s1 = (w as f64 * 0.3, h as f64 * 0.5);
        let s2 = (w as f64 * 0.7, h as f64 * 0.4);
        let wave_k = params.scale * 0.2;

        for y in 0..h {
            for x in 0..w {
                let d1 = ((x as f64 - s1.0).powi(2) + (y as f64 - s1.1).powi(2)).sqrt();
                let d2 = ((x as f64 - s2.0).powi(2) + (y as f64 - s2.1).powi(2)).sqrt();

                let w1 = (d1 * wave_k).cos();
                let w2 = (d2 * wave_k).cos();
                let interference = (w1 + w2) * 0.5; // -1..1

                // Noise distortion
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;
                let noise = fbm(nx, ny, 3, params.seed) * params.variation * 0.2;

                let t = ((interference + noise) * 0.5 + 0.5).clamp(0.0, 1.0);
                image[y][x] = lerp_color(params.color_a, params.color_b, t);
            }
        }
        image
    }

    /// Clouds: FBM cloud formation.
    pub fn clouds(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;
                let v = fbm(nx, ny, 6, params.seed);
                // Cloud threshold: values above 0.0 are "cloud"
                let t = (v * 1.5 + 0.5).clamp(0.0, 1.0);
                image[y][x] = lerp_color(params.color_a, params.color_b, t);
            }
        }
        image
    }

    /// Lava: high-contrast orange/black FBM.
    pub fn lava(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;
                let t_val = turb(nx, ny, params.seed, 5);
                // High contrast: abs(fbm) → lava glow
                let t = (t_val * 2.0).clamp(0.0, 1.0);
                // color_a should be orange/red, color_b black
                image[y][x] = lerp_color(params.color_b, params.color_a, t);
            }
        }
        image
    }

    /// Concrete: fine-grain noise with subtle variation.
    pub fn concrete(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale * 4.0;
                let ny = y as f64 / h as f64 * params.scale * 4.0;
                let v = fbm(nx, ny, 5, params.seed);
                let t = (v * params.variation * 0.5 + 0.5).clamp(0.0, 1.0);
                image[y][x] = lerp_color(params.color_a, params.color_b, t);
            }
        }
        image
    }

    /// Sand: soft dunes with fine ripple noise.
    pub fn sand(params: &TextureParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;

                // Dune: low-frequency FBM
                let dune = fbm(nx * 0.5, ny * 0.5, 3, params.seed);
                // Ripple: high-frequency in perpendicular direction
                let ripple = ((nx * 8.0 + dune * 3.0).sin() + 1.0) * 0.5 * 0.15;

                let t = ((dune + 1.0) * 0.5 * (1.0 - params.variation) + ripple).clamp(0.0, 1.0);
                image[y][x] = lerp_color(params.color_a, params.color_b, t);
            }
        }
        image
    }

    /// Linear interpolation between colors.
    pub fn lerp_color(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
        lerp_color(a, b, t)
    }

    /// Turbulence: sum of abs(FBM) over octaves.
    pub fn turb(x: f64, y: f64, seed: u64, octaves: u8) -> f64 {
        turb(x, y, seed, octaves)
    }
}

// -----------------------------------------------------------------------
// Module-level helpers
// -----------------------------------------------------------------------

pub(crate) fn turb(x: f64, y: f64, seed: u64, octaves: u8) -> f64 {
    let mut value = 0.0f64;
    let mut amplitude = 0.5f64;
    let mut frequency = 1.0f64;

    for oct in 0..octaves {
        let oct_seed = seed.wrapping_add((oct as u64).wrapping_mul(0x9E3779B97F4A7C15));
        value += amplitude * fbm(x * frequency, y * frequency, 1, oct_seed).abs();
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

fn simple_hash(a: u64, b: u64, seed: u64) -> u64 {
    let mut h = seed
        .wrapping_add(a.wrapping_mul(0x517CC1B727220A95))
        .wrapping_add(b.wrapping_mul(0x6C62272E07BB0142));
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51AFD7ED558CCD);
    h ^= h >> 33;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params(tt: TextureType) -> TextureParams {
        TextureParams {
            texture_type: tt,
            width: 16,
            height: 16,
            seed: 42,
            scale: 2.0,
            color_a: [139, 90, 43],
            color_b: [200, 160, 100],
            variation: 0.5,
        }
    }

    #[test]
    fn test_wood_dimensions() {
        let img = TextureSynthesizer::wood(&default_params(TextureType::Wood));
        assert_eq!(img.len(), 16);
        assert_eq!(img[0].len(), 16);
    }

    #[test]
    fn test_marble_dimensions() {
        let img = TextureSynthesizer::marble(&default_params(TextureType::Marble));
        assert_eq!(img.len(), 16);
    }

    #[test]
    fn test_brick_dimensions() {
        let img = TextureSynthesizer::brick(&default_params(TextureType::Brick));
        assert_eq!(img.len(), 16);
        assert_eq!(img[0].len(), 16);
    }

    #[test]
    fn test_all_types_synthesize() {
        let types = [
            TextureType::Wood,
            TextureType::Marble,
            TextureType::Brick,
            TextureType::Fabric,
            TextureType::Metal,
            TextureType::Water,
            TextureType::Clouds,
            TextureType::Lava,
            TextureType::Concrete,
            TextureType::Sand,
        ];
        for tt in types {
            let mut p = default_params(tt);
            let img = TextureSynthesizer::synthesize(&p);
            assert_eq!(img.len(), 16, "height mismatch");
            assert_eq!(img[0].len(), 16, "width mismatch");
        }
    }

    #[test]
    fn test_turb_positive() {
        let v = TextureSynthesizer::turb(1.0, 2.0, 7, 4);
        assert!(v >= 0.0, "turbulence should be non-negative: {}", v);
    }

    #[test]
    fn test_lerp_color() {
        let c = TextureSynthesizer::lerp_color([0, 0, 0], [255, 255, 255], 0.5);
        assert!(c[0] > 100 && c[0] < 160);
    }
}
