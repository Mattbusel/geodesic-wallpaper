//! Noise-based generative art with color palettes.

/// A color palette for mapping scalar values to RGB colors.
#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub colors: Vec<[u8; 3]>,
    pub name: String,
}

impl ColorPalette {
    /// Build a palette by sampling `n` colors from gradient stops.
    ///
    /// Each stop is `([r,g,b], position)` where position ∈ [0,1].
    pub fn from_stops(stops: Vec<([u8; 3], f64)>, n: usize) -> Self {
        assert!(!stops.is_empty());
        let n = n.max(2);
        let mut colors = Vec::with_capacity(n);

        for i in 0..n {
            let t = i as f64 / (n - 1) as f64;
            colors.push(Self::sample_stops(&stops, t));
        }

        ColorPalette {
            colors,
            name: String::new(),
        }
    }

    fn sample_stops(stops: &[([u8; 3], f64)], t: f64) -> [u8; 3] {
        if stops.len() == 1 {
            return stops[0].0;
        }
        // Find surrounding stops
        let mut lo = &stops[0];
        let mut hi = &stops[stops.len() - 1];
        for i in 0..stops.len().saturating_sub(1) {
            if t >= stops[i].1 && t <= stops[i + 1].1 {
                lo = &stops[i];
                hi = &stops[i + 1];
                break;
            }
        }
        let range = hi.1 - lo.1;
        let local_t = if range < 1e-9 {
            0.0
        } else {
            (t - lo.1) / range
        };
        lerp_color(lo.0, hi.0, local_t.clamp(0.0, 1.0))
    }

    /// Sample a color at position `t ∈ [0,1]`.
    pub fn sample(&self, t: f64) -> [u8; 3] {
        if self.colors.is_empty() {
            return [0, 0, 0];
        }
        let t = t.clamp(0.0, 1.0);
        let scaled = t * (self.colors.len() - 1) as f64;
        let lo = scaled.floor() as usize;
        let hi = (lo + 1).min(self.colors.len() - 1);
        let frac = scaled.fract();
        lerp_color(self.colors[lo], self.colors[hi], frac)
    }

    /// Get one of the built-in palettes by name.
    pub fn built_in(name: &str) -> Option<Self> {
        let (stops, n): (Vec<([u8; 3], f64)>, usize) = match name {
            "sunset" => (
                vec![
                    ([20, 10, 40], 0.0),
                    ([255, 80, 0], 0.5),
                    ([255, 220, 100], 1.0),
                ],
                32,
            ),
            "ocean" => (
                vec![
                    ([0, 20, 80], 0.0),
                    ([0, 100, 200], 0.5),
                    ([150, 220, 255], 1.0),
                ],
                32,
            ),
            "forest" => (
                vec![
                    ([10, 40, 10], 0.0),
                    ([30, 120, 30], 0.5),
                    ([180, 220, 100], 1.0),
                ],
                32,
            ),
            "neon" => (
                vec![
                    ([0, 0, 0], 0.0),
                    ([255, 0, 200], 0.33),
                    ([0, 255, 200], 0.66),
                    ([255, 255, 0], 1.0),
                ],
                32,
            ),
            "pastel" => (
                vec![
                    ([255, 200, 220], 0.0),
                    ([200, 220, 255], 0.5),
                    ([220, 255, 200], 1.0),
                ],
                32,
            ),
            "monochrome" => (
                vec![
                    ([0, 0, 0], 0.0),
                    ([128, 128, 128], 0.5),
                    ([255, 255, 255], 1.0),
                ],
                32,
            ),
            "fire" => (
                vec![
                    ([0, 0, 0], 0.0),
                    ([180, 0, 0], 0.3),
                    ([255, 120, 0], 0.6),
                    ([255, 255, 100], 1.0),
                ],
                32,
            ),
            _ => return None,
        };
        let mut palette = Self::from_stops(stops, n);
        palette.name = name.to_string();
        Some(palette)
    }
}

/// Available generative art styles.
#[derive(Debug, Clone, PartialEq)]
pub enum ArtStyle {
    NoiseLandscape,
    FlowFieldArt,
    OrganicCells,
    CosmicDust,
    CrystalGrowth,
    MarbledPaper,
}

/// Parameters controlling generative art rendering.
#[derive(Debug, Clone)]
pub struct GenerativeParams {
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub palette: ColorPalette,
    pub scale: f64,
    pub octaves: u8,
    pub time_offset: f64,
    pub style: ArtStyle,
}

/// Renders generative art images.
pub struct GenerativeArtist;

impl GenerativeArtist {
    /// Dispatch to the appropriate renderer based on style.
    pub fn render(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        match params.style {
            ArtStyle::NoiseLandscape => Self::noise_landscape(params),
            ArtStyle::FlowFieldArt => Self::flow_field_art(params),
            ArtStyle::OrganicCells => Self::organic_cells(params),
            ArtStyle::CosmicDust => Self::cosmic_dust(params),
            ArtStyle::CrystalGrowth => Self::crystal_growth(params),
            ArtStyle::MarbledPaper => Self::marbled_paper(params),
        }
    }

    /// FBM noise → elevation → palette mapping.
    pub fn noise_landscape(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];
        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale + params.time_offset;
                let ny = y as f64 / h as f64 * params.scale;
                let v = fbm(nx, ny, params.octaves, params.seed);
                let t = (v + 1.0) * 0.5; // map [-1,1] → [0,1]
                image[y][x] = params.palette.sample(t);
            }
        }
        image
    }

    /// Curl-noise streamlines on a colored background.
    pub fn flow_field_art(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[20u8, 20u8, 40u8]; w]; h];

        // Seed particle positions
        let num_particles = (w * h / 64).max(50);
        let mut rng = params.seed;

        for _ in 0..num_particles {
            let mut px = next_rand_f64(&mut rng) * w as f64;
            let mut py = next_rand_f64(&mut rng) * h as f64;

            for step in 0..60 {
                let ix = px as usize;
                let iy = py as usize;
                if ix >= w || iy >= h {
                    break;
                }

                let t_color = step as f64 / 60.0;
                let color = params.palette.sample(t_color);

                // Alpha blend into image
                let bg = image[iy][ix];
                image[iy][ix] = [
                    blend_u8(bg[0], color[0], 0.3),
                    blend_u8(bg[1], color[1], 0.3),
                    blend_u8(bg[2], color[2], 0.3),
                ];

                // Curl of noise field gives flow direction
                let eps = 0.01;
                let nx = px / w as f64 * params.scale;
                let ny = py / h as f64 * params.scale;
                let dydx = (fbm(nx + eps, ny, params.octaves, params.seed)
                    - fbm(nx - eps, ny, params.octaves, params.seed))
                    / (2.0 * eps);
                let dxdy = (fbm(nx, ny + eps, params.octaves, params.seed)
                    - fbm(nx, ny - eps, params.octaves, params.seed))
                    / (2.0 * eps);
                let curl_x = dydx;
                let curl_y = -dxdy;

                px += curl_x * 2.0;
                py += curl_y * 2.0;
            }
        }
        image
    }

    /// Voronoi cells with organic noise distortion.
    pub fn organic_cells(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let num_seeds = 20usize;
        let mut rng = params.seed;

        // Generate seed points
        let seeds: Vec<(f64, f64)> = (0..num_seeds)
            .map(|_| {
                (
                    next_rand_f64(&mut rng) * w as f64,
                    next_rand_f64(&mut rng) * h as f64,
                )
            })
            .collect();

        let mut image = vec![vec![[0u8; 3]; w]; h];
        for y in 0..h {
            for x in 0..w {
                // Noise-distorted coordinates
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;
                let dx = fbm(nx + 1.7, ny + 9.2, params.octaves, params.seed) * 20.0;
                let dy = fbm(nx + 8.3, ny + 2.8, params.octaves, params.seed) * 20.0;

                let px = x as f64 + dx;
                let py = y as f64 + dy;

                // Find nearest seed
                let (nearest_idx, _) =
                    seeds
                        .iter()
                        .enumerate()
                        .fold((0, f64::MAX), |(bi, bd), (i, &(sx, sy))| {
                            let d = (px - sx).powi(2) + (py - sy).powi(2);
                            if d < bd {
                                (i, d)
                            } else {
                                (bi, bd)
                            }
                        });

                let t = nearest_idx as f64 / num_seeds as f64;
                image[y][x] = params.palette.sample(t);
            }
        }
        image
    }

    /// Star field with nebula noise.
    pub fn cosmic_dust(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8, 0u8, 10u8]; w]; h];

        // Nebula background
        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;
                let v = fbm(nx, ny, params.octaves, params.seed);
                let t = (v + 1.0) * 0.5;
                if t > 0.55 {
                    let nebula = params.palette.sample(t);
                    let alpha = (t - 0.55) * 2.0;
                    let bg = image[y][x];
                    image[y][x] = [
                        blend_u8(bg[0], nebula[0], alpha),
                        blend_u8(bg[1], nebula[1], alpha),
                        blend_u8(bg[2], nebula[2], alpha),
                    ];
                }
            }
        }

        // Stars
        let mut rng = params.seed;
        let num_stars = (w * h / 200).max(100);
        for _ in 0..num_stars {
            let sx = (next_rand_f64(&mut rng) * w as f64) as usize;
            let sy = (next_rand_f64(&mut rng) * h as f64) as usize;
            let brightness = (next_rand_f64(&mut rng) * 200.0 + 55.0) as u8;
            if sx < w && sy < h {
                image[sy][sx] = [brightness, brightness, brightness];
            }
        }

        image
    }

    /// DLA-like crystal growth from random seeds.
    pub fn crystal_growth(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut grid = vec![vec![false; w]; h];
        let mut image = vec![vec![[0u8; 3]; w]; h];

        let mut rng = params.seed;

        // Initial seeds
        let num_seeds = 5usize;
        let mut seeds: Vec<(usize, usize)> = (0..num_seeds)
            .map(|_| {
                let sx = (next_rand_f64(&mut rng) * w as f64) as usize % w;
                let sy = (next_rand_f64(&mut rng) * h as f64) as usize % h;
                grid[sy][sx] = true;
                (sx, sy)
            })
            .collect();

        // DLA: random walkers stick to seeds
        let num_walkers = (w * h / 20).min(5000);
        for step in 0..num_walkers {
            let mut wx = (next_rand_f64(&mut rng) * w as f64) as usize % w;
            let mut wy = (next_rand_f64(&mut rng) * h as f64) as usize % h;

            for _ in 0..100 {
                // Random walk step
                let dx = (next_rand_f64(&mut rng) * 3.0) as i64 - 1;
                let dy = (next_rand_f64(&mut rng) * 3.0) as i64 - 1;
                wx = ((wx as i64 + dx).rem_euclid(w as i64)) as usize;
                wy = ((wy as i64 + dy).rem_euclid(h as i64)) as usize;

                // Check adjacency to existing crystal
                let neighbors = [
                    (wx.wrapping_sub(1), wy),
                    (wx + 1, wy),
                    (wx, wy.wrapping_sub(1)),
                    (wx, wy + 1),
                ];
                let adjacent = neighbors
                    .iter()
                    .any(|&(nx, ny)| nx < w && ny < h && grid[ny][nx]);

                if adjacent && !grid[wy][wx] {
                    grid[wy][wx] = true;
                    seeds.push((wx, wy));
                    let t = step as f64 / num_walkers as f64;
                    image[wy][wx] = params.palette.sample(t);
                    break;
                }
            }
        }

        image
    }

    /// Sine + domain warping for marble-vein effect.
    pub fn marbled_paper(params: &GenerativeParams) -> Vec<Vec<[u8; 3]>> {
        let (w, h) = (params.width as usize, params.height as usize);
        let mut image = vec![vec![[0u8; 3]; w]; h];

        for y in 0..h {
            for x in 0..w {
                let nx = x as f64 / w as f64 * params.scale;
                let ny = y as f64 / h as f64 * params.scale;

                // Domain warping: distort the sampling position
                let qx = fbm(nx, ny, params.octaves, params.seed);
                let qy = fbm(nx + 5.2, ny + 1.3, params.octaves, params.seed ^ 0xDEAD);

                let rx = fbm(
                    nx + 4.0 * qx + 1.7,
                    ny + 4.0 * qy + 9.2,
                    params.octaves,
                    params.seed ^ 0xBEEF,
                );

                // Marble veins from sine
                let vein = ((nx + 3.0 * rx + params.time_offset).sin() + 1.0) * 0.5;
                image[y][x] = params.palette.sample(vein);
            }
        }
        image
    }

    /// 2D Perlin-style gradient noise.
    pub fn perlin_2d(x: f64, y: f64, seed: u64) -> f64 {
        let xi = x.floor() as i64;
        let yi = y.floor() as i64;
        let xf = x - x.floor();
        let yf = y - y.floor();

        let u = fade(xf);
        let v = fade(yf);

        let aa = gradient(hash(xi, yi, seed), xf, yf);
        let ba = gradient(hash(xi + 1, yi, seed), xf - 1.0, yf);
        let ab = gradient(hash(xi, yi + 1, seed), xf, yf - 1.0);
        let bb = gradient(hash(xi + 1, yi + 1, seed), xf - 1.0, yf - 1.0);

        let x1 = lerp(aa, ba, u);
        let x2 = lerp(ab, bb, u);
        lerp(x1, x2, v)
    }

    /// Fractional Brownian Motion built from `octaves` layers of perlin_2d.
    pub fn fbm(x: f64, y: f64, octaves: u8, seed: u64) -> f64 {
        fbm(x, y, octaves, seed)
    }
}

// -----------------------------------------------------------------------
// Module-level helpers (shared with texture_synthesizer)
// -----------------------------------------------------------------------

pub(crate) fn fbm(x: f64, y: f64, octaves: u8, seed: u64) -> f64 {
    let mut value = 0.0f64;
    let mut amplitude = 0.5f64;
    let mut frequency = 1.0f64;

    for oct in 0..octaves {
        let oct_seed = seed.wrapping_add((oct as u64).wrapping_mul(0x9E3779B97F4A7C15));
        value += amplitude * GenerativeArtist::perlin_2d(x * frequency, y * frequency, oct_seed);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

pub(crate) fn lerp_color(a: [u8; 3], b: [u8; 3], t: f64) -> [u8; 3] {
    [
        lerp(a[0] as f64, b[0] as f64, t) as u8,
        lerp(a[1] as f64, b[1] as f64, t) as u8,
        lerp(a[2] as f64, b[2] as f64, t) as u8,
    ]
}

fn hash(xi: i64, yi: i64, seed: u64) -> u64 {
    let mut h = seed
        .wrapping_add((xi as u64).wrapping_mul(0x517CC1B727220A95))
        .wrapping_add((yi as u64).wrapping_mul(0x6C62272E07BB0142));
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51AFD7ED558CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CEB9FE1A85EC53);
    h ^= h >> 33;
    h
}

fn gradient(hash: u64, x: f64, y: f64) -> f64 {
    match hash & 3 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        _ => -x - y,
    }
}

fn blend_u8(a: u8, b: u8, t: f64) -> u8 {
    (a as f64 + (b as f64 - a as f64) * t.clamp(0.0, 1.0)) as u8
}

fn next_rand_f64(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as f64) / (u64::MAX as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_sample() {
        let p = ColorPalette::built_in("sunset").unwrap();
        let color = p.sample(0.5);
        // Just check it's a valid RGB triple
        assert!(color.iter().all(|&c| c <= 255));
    }

    #[test]
    fn test_palette_built_in_names() {
        for name in &[
            "sunset",
            "ocean",
            "forest",
            "neon",
            "pastel",
            "monochrome",
            "fire",
        ] {
            assert!(
                ColorPalette::built_in(name).is_some(),
                "missing palette: {}",
                name
            );
        }
        assert!(ColorPalette::built_in("nonexistent").is_none());
    }

    #[test]
    fn test_noise_landscape_dimensions() {
        let params = GenerativeParams {
            width: 16,
            height: 8,
            seed: 42,
            palette: ColorPalette::built_in("ocean").unwrap(),
            scale: 2.0,
            octaves: 4,
            time_offset: 0.0,
            style: ArtStyle::NoiseLandscape,
        };
        let img = GenerativeArtist::noise_landscape(&params);
        assert_eq!(img.len(), 8);
        assert_eq!(img[0].len(), 16);
    }

    #[test]
    fn test_fbm_bounded() {
        for i in 0..20 {
            let v = fbm(i as f64 * 0.3, i as f64 * 0.17, 4, 12345);
            assert!(v.abs() < 2.0, "FBM value too large: {}", v);
        }
    }

    #[test]
    fn test_marbled_paper() {
        let params = GenerativeParams {
            width: 8,
            height: 8,
            seed: 99,
            palette: ColorPalette::built_in("fire").unwrap(),
            scale: 3.0,
            octaves: 3,
            time_offset: 0.0,
            style: ArtStyle::MarbledPaper,
        };
        let img = GenerativeArtist::marbled_paper(&params);
        assert_eq!(img.len(), 8);
    }

    #[test]
    fn test_render_dispatch() {
        let params = GenerativeParams {
            width: 8,
            height: 8,
            seed: 1,
            palette: ColorPalette::built_in("neon").unwrap(),
            scale: 2.0,
            octaves: 2,
            time_offset: 0.0,
            style: ArtStyle::FlowFieldArt,
        };
        let img = GenerativeArtist::render(&params);
        assert_eq!(img.len(), 8);
        assert_eq!(img[0].len(), 8);
    }
}
