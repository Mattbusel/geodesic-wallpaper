//! Procedural terrain heightmap generation with fBm noise, terrain classification,
//! statistics, and simple hydraulic erosion.

// ---------------------------------------------------------------------------
// TerrainConfig
// ---------------------------------------------------------------------------

/// Configuration parameters for heightmap generation.
#[derive(Debug, Clone)]
pub struct TerrainConfig {
    pub width: u32,
    pub height: u32,
    /// Number of fBm octaves.
    pub octaves: u8,
    /// Amplitude multiplier per octave (0 < persistence < 1).
    pub persistence: f64,
    /// Frequency multiplier per octave (lacunarity > 1).
    pub lacunarity: f64,
    /// Base frequency scale.
    pub scale: f64,
    /// Seed for the gradient-noise permutation table.
    pub seed: u64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 256,
            octaves: 6,
            persistence: 0.5,
            lacunarity: 2.0,
            scale: 0.01,
            seed: 42,
        }
    }
}

// ---------------------------------------------------------------------------
// Gradient noise (Perlin-like)
// ---------------------------------------------------------------------------

/// Build a shuffled permutation table from the given seed.
fn build_perm(seed: u64) -> [u8; 512] {
    let mut p: [u8; 256] = core::array::from_fn(|i| i as u8);
    // Fisher-Yates shuffle with a simple LCG.
    let mut state = seed.wrapping_add(1);
    for i in (1..256usize).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (state >> 33) as usize % (i + 1);
        p.swap(i, j);
    }
    let mut perm = [0u8; 512];
    for i in 0..256 {
        perm[i] = p[i];
        perm[i + 256] = p[i];
    }
    perm
}

/// Quintic fade curve.
#[inline]
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Linear interpolation.
#[inline]
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

/// Gradient selection from hash value and (x, y) offsets.
#[inline]
fn grad2(hash: u8, x: f64, y: f64) -> f64 {
    match hash & 7 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y,
    }
}

/// Sample 2-D gradient noise in [-1, 1].
fn noise2(x: f64, y: f64, perm: &[u8; 512]) -> f64 {
    let xi = x.floor() as i64 & 255;
    let yi = y.floor() as i64 & 255;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let u = fade(xf);
    let v = fade(yf);

    let aa = perm[(perm[xi as usize] as usize + yi as usize) & 255];
    let ab = perm[(perm[xi as usize] as usize + (yi + 1) as usize) & 255];
    let ba = perm[(perm[(xi + 1) as usize & 255] as usize + yi as usize) & 255];
    let bb = perm[(perm[(xi + 1) as usize & 255] as usize + (yi + 1) as usize) & 255];

    let x1 = lerp(grad2(aa, xf, yf), grad2(ba, xf - 1.0, yf), u);
    let x2 = lerp(grad2(ab, xf, yf - 1.0), grad2(bb, xf - 1.0, yf - 1.0), u);
    lerp(x1, x2, v)
}

/// Fractal Brownian Motion: accumulate `octaves` layers of gradient noise.
fn fbm(x: f64, y: f64, config: &TerrainConfig, perm: &[u8; 512]) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = config.scale;
    let mut max_value = 0.0;

    for _ in 0..config.octaves {
        value += noise2(x * frequency, y * frequency, perm) * amplitude;
        max_value += amplitude;
        amplitude *= config.persistence;
        frequency *= config.lacunarity;
    }
    // Normalise to [-1, 1].
    if max_value > 0.0 {
        value / max_value
    } else {
        0.0
    }
}

// ---------------------------------------------------------------------------
// Heightmap generation
// ---------------------------------------------------------------------------

/// Generate a heightmap normalised to [0, 1].
pub fn generate_heightmap(config: &TerrainConfig) -> Vec<Vec<f64>> {
    let perm = build_perm(config.seed);

    // Raw fBm values are in approximately [-1, 1]; collect min/max to normalise.
    let mut raw: Vec<Vec<f64>> = (0..config.height)
        .map(|row| {
            (0..config.width)
                .map(|col| fbm(col as f64, row as f64, config, &perm))
                .collect()
        })
        .collect();

    let mut min_v = f64::MAX;
    let mut max_v = f64::MIN;
    for row in &raw {
        for &v in row {
            if v < min_v {
                min_v = v;
            }
            if v > max_v {
                max_v = v;
            }
        }
    }
    let range = max_v - min_v;
    if range > 1e-10 {
        for row in &mut raw {
            for v in row.iter_mut() {
                *v = (*v - min_v) / range;
            }
        }
    }
    raw
}

// ---------------------------------------------------------------------------
// TerrainClassifier
// ---------------------------------------------------------------------------

/// Terrain classification based on height thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainClass {
    Ocean,
    Beach,
    Grass,
    Mountain,
    Snow,
}

impl TerrainClass {
    /// Classify a normalised height value.
    pub fn classify(height: f64) -> Self {
        if height < 0.3 {
            TerrainClass::Ocean
        } else if height < 0.4 {
            TerrainClass::Beach
        } else if height < 0.7 {
            TerrainClass::Grass
        } else if height < 0.9 {
            TerrainClass::Mountain
        } else {
            TerrainClass::Snow
        }
    }
}

/// Classify an entire heightmap cell-by-cell.
pub struct TerrainClassifier;

impl TerrainClassifier {
    pub fn classify_map(heightmap: &Vec<Vec<f64>>) -> Vec<Vec<TerrainClass>> {
        heightmap
            .iter()
            .map(|row| row.iter().map(|&h| TerrainClass::classify(h)).collect())
            .collect()
    }
}

// ---------------------------------------------------------------------------
// TerrainStats
// ---------------------------------------------------------------------------

/// Aggregate statistics for a heightmap.
#[derive(Debug, Clone)]
pub struct TerrainStats {
    pub min_height: f64,
    pub max_height: f64,
    pub mean_height: f64,
    /// Fraction of cells classified as Ocean (height < 0.3).
    pub water_fraction: f64,
}

impl TerrainStats {
    /// Compute statistics from a heightmap.
    pub fn compute(heightmap: &Vec<Vec<f64>>) -> Self {
        let mut min_v = f64::MAX;
        let mut max_v = f64::MIN;
        let mut sum = 0.0;
        let mut water = 0u64;
        let mut count = 0u64;

        for row in heightmap {
            for &h in row {
                if h < min_v {
                    min_v = h;
                }
                if h > max_v {
                    max_v = h;
                }
                sum += h;
                count += 1;
                if h < 0.3 {
                    water += 1;
                }
            }
        }

        let (min_height, max_height, mean_height, water_fraction) = if count == 0 {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            (
                min_v,
                max_v,
                sum / count as f64,
                water as f64 / count as f64,
            )
        };

        Self {
            min_height,
            max_height,
            mean_height,
            water_fraction,
        }
    }
}

// ---------------------------------------------------------------------------
// Hydraulic erosion
// ---------------------------------------------------------------------------

/// Simple hydraulic erosion: for each iteration, water flows from each cell to
/// its lowest neighbour, carrying sediment and depositing it there.
pub fn erode(heightmap: &mut Vec<Vec<f64>>, iterations: u32) {
    let rows = heightmap.len();
    if rows == 0 {
        return;
    }
    let cols = heightmap[0].len();
    if cols == 0 {
        return;
    }

    // Erosion and deposition rates.
    const EROSION_RATE: f64 = 0.01;
    const DEPOSITION_RATE: f64 = 0.005;

    for _ in 0..iterations {
        // Snapshot to avoid in-place mutation artefacts.
        let snapshot: Vec<Vec<f64>> = heightmap.clone();

        for r in 0..rows {
            for c in 0..cols {
                let h = snapshot[r][c];

                // Find the lowest neighbour (4-connectivity).
                let neighbours = [
                    if r > 0 { Some((r - 1, c)) } else { None },
                    if r + 1 < rows { Some((r + 1, c)) } else { None },
                    if c > 0 { Some((r, c - 1)) } else { None },
                    if c + 1 < cols { Some((r, c + 1)) } else { None },
                ];

                let lowest = neighbours
                    .iter()
                    .flatten()
                    .min_by(|&&(ar, ac), &&(br, bc)| {
                        snapshot[ar][ac].partial_cmp(&snapshot[br][bc]).unwrap()
                    })
                    .copied();

                if let Some((nr, nc)) = lowest {
                    let nh = snapshot[nr][nc];
                    if nh < h {
                        let diff = (h - nh) * EROSION_RATE;
                        heightmap[r][c] -= diff;
                        heightmap[nr][nc] += diff * (1.0 - DEPOSITION_RATE);
                    }
                }
            }
        }
        // Clamp to [0, 1].
        for row in heightmap.iter_mut() {
            for v in row.iter_mut() {
                *v = v.clamp(0.0, 1.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> TerrainConfig {
        TerrainConfig {
            width: 16,
            height: 16,
            ..Default::default()
        }
    }

    #[test]
    fn heightmap_dimensions() {
        let cfg = small_config();
        let hm = generate_heightmap(&cfg);
        assert_eq!(hm.len(), 16);
        assert_eq!(hm[0].len(), 16);
    }

    #[test]
    fn heightmap_values_in_range() {
        let cfg = small_config();
        let hm = generate_heightmap(&cfg);
        for row in &hm {
            for &v in row {
                assert!(v >= 0.0, "value below 0: {}", v);
                assert!(v <= 1.0, "value above 1: {}", v);
            }
        }
    }

    #[test]
    fn heightmap_has_variation() {
        let cfg = small_config();
        let hm = generate_heightmap(&cfg);
        let min = hm.iter().flatten().cloned().fold(f64::MAX, f64::min);
        let max = hm.iter().flatten().cloned().fold(f64::MIN, f64::max);
        assert!(max - min > 0.01, "No variation in heightmap");
    }

    #[test]
    fn different_seeds_produce_different_maps() {
        let cfg1 = TerrainConfig {
            seed: 1,
            ..small_config()
        };
        let cfg2 = TerrainConfig {
            seed: 2,
            ..small_config()
        };
        let hm1 = generate_heightmap(&cfg1);
        let hm2 = generate_heightmap(&cfg2);
        assert_ne!(hm1[0][0].to_bits(), hm2[0][0].to_bits());
    }

    #[test]
    fn terrain_classify_thresholds() {
        assert_eq!(TerrainClass::classify(0.1), TerrainClass::Ocean);
        assert_eq!(TerrainClass::classify(0.35), TerrainClass::Beach);
        assert_eq!(TerrainClass::classify(0.55), TerrainClass::Grass);
        assert_eq!(TerrainClass::classify(0.80), TerrainClass::Mountain);
        assert_eq!(TerrainClass::classify(0.95), TerrainClass::Snow);
    }

    #[test]
    fn terrain_classify_map_dimensions() {
        let hm = vec![vec![0.1, 0.5], vec![0.8, 0.95]];
        let cm = TerrainClassifier::classify_map(&hm);
        assert_eq!(cm.len(), 2);
        assert_eq!(cm[0].len(), 2);
        assert_eq!(cm[0][0], TerrainClass::Ocean);
        assert_eq!(cm[1][1], TerrainClass::Snow);
    }

    #[test]
    fn terrain_stats_basic() {
        let hm = vec![vec![0.0_f64, 0.5], vec![1.0, 0.2]];
        let s = TerrainStats::compute(&hm);
        assert!((s.min_height - 0.0).abs() < 1e-9);
        assert!((s.max_height - 1.0).abs() < 1e-9);
        assert!((s.mean_height - 0.425).abs() < 1e-9);
        // 0.0 and 0.2 are < 0.3 → water_fraction = 2/4 = 0.5
        assert!((s.water_fraction - 0.5).abs() < 1e-9);
    }

    #[test]
    fn terrain_stats_empty() {
        let hm: Vec<Vec<f64>> = vec![];
        let s = TerrainStats::compute(&hm);
        assert_eq!(s.min_height, 0.0);
        assert_eq!(s.max_height, 0.0);
    }

    #[test]
    fn erode_does_not_produce_out_of_range_values() {
        let cfg = small_config();
        let mut hm = generate_heightmap(&cfg);
        erode(&mut hm, 5);
        for row in &hm {
            for &v in row {
                assert!(v >= 0.0 && v <= 1.0, "out of range after erosion: {}", v);
            }
        }
    }

    #[test]
    fn erode_changes_heightmap() {
        let cfg = small_config();
        let hm_orig = generate_heightmap(&cfg);
        let mut hm = hm_orig.clone();
        erode(&mut hm, 3);
        let changed = hm_orig
            .iter()
            .flatten()
            .zip(hm.iter().flatten())
            .any(|(a, b)| (a - b).abs() > 1e-12);
        assert!(changed, "erosion did not change the heightmap");
    }
}
