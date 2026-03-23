//! M.C. Escher-style tessellation pattern generator.
//!
//! Generates parallelogram lattices, hexagonal tilings, simplified lizard
//! patterns, and wallpaper-group symmetry transforms.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Which Escher-inspired tessellation style to produce.
#[derive(Debug, Clone, PartialEq)]
pub enum TessellationType {
    RegularDivision,
    LizardPattern,
    FishPattern,
    BirdPattern,
    SymmetryGroup(u8),
}

/// A single tile shape: a polygon with a fill colour.
#[derive(Debug, Clone)]
pub struct TileShape {
    pub vertices: Vec<(f64, f64)>,
    pub color: [u8; 3],
}

// ---------------------------------------------------------------------------
// EscherGenerator
// ---------------------------------------------------------------------------

pub struct EscherGenerator;

impl EscherGenerator {
    pub fn new() -> Self {
        Self
    }

    // -----------------------------------------------------------------------
    // Parallelogram tiling
    // -----------------------------------------------------------------------

    /// Generate a parallelogram lattice defined by basis vectors `a` and `b`.
    ///
    /// `cols` × `rows` tiles are generated, with colours cycling through
    /// `colors`.
    pub fn generate_parallelogram_tiling(
        &self,
        a: (f64, f64),
        b: (f64, f64),
        cols: u32,
        rows: u32,
        colors: &[[u8; 3]],
    ) -> Vec<TileShape> {
        if colors.is_empty() {
            return vec![];
        }
        let mut tiles = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let ox = col as f64 * a.0 + row as f64 * b.0;
                let oy = col as f64 * a.1 + row as f64 * b.1;

                let vertices = vec![
                    (ox, oy),
                    (ox + a.0, oy + a.1),
                    (ox + a.0 + b.0, oy + a.1 + b.1),
                    (ox + b.0, oy + b.1),
                ];

                let color_idx = (row * cols + col) as usize % colors.len();
                tiles.push(TileShape {
                    vertices,
                    color: colors[color_idx],
                });
            }
        }
        tiles
    }

    // -----------------------------------------------------------------------
    // Hexagonal tiling
    // -----------------------------------------------------------------------

    /// Generate a regular hexagon tessellation.
    ///
    /// Each hexagon is centred at a lattice point derived from `center`,
    /// `radius`, and hex grid offsets for `cols` × `rows` cells.
    pub fn generate_hexagonal_tiling(
        &self,
        center: (f64, f64),
        radius: f64,
        cols: u32,
        rows: u32,
        colors: &[[u8; 3]],
    ) -> Vec<TileShape> {
        if colors.is_empty() {
            return vec![];
        }
        let w = radius * 3.0_f64.sqrt();       // horizontal spacing
        let h = radius * 1.5;                   // vertical spacing

        let mut tiles = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let cx = center.0 + col as f64 * w + if row % 2 == 1 { w / 2.0 } else { 0.0 };
                let cy = center.1 + row as f64 * h;

                let vertices: Vec<(f64, f64)> = (0..6)
                    .map(|i| {
                        let angle = PI / 3.0 * i as f64 - PI / 6.0;
                        (cx + radius * angle.cos(), cy + radius * angle.sin())
                    })
                    .collect();

                let color_idx = (row * cols + col) as usize % colors.len();
                tiles.push(TileShape {
                    vertices,
                    color: colors[color_idx],
                });
            }
        }
        tiles
    }

    // -----------------------------------------------------------------------
    // Lizard pattern
    // -----------------------------------------------------------------------

    /// Generate a simplified Escher-inspired lizard pattern.
    ///
    /// Produces interlocking L-shaped tiles on a grid, with each tile
    /// rotated 0°, 90°, 180°, or 270° based on a seeded pattern.
    pub fn generate_lizard_pattern(
        &self,
        width: u32,
        height: u32,
        scale: f64,
        seed: u64,
    ) -> Vec<Vec<[u8; 3]>> {
        let w = width as usize;
        let h = height as usize;
        let mut canvas = vec![vec![[30u8, 30u8, 30u8]; w]; h];

        let tile_size = (scale * 20.0).max(4.0) as usize;
        let colors: [[u8; 3]; 4] = [
            [200, 80, 50],
            [50, 160, 80],
            [60, 80, 200],
            [200, 160, 50],
        ];

        let mut rng = seed;
        let cols = (w / tile_size).max(1);
        let rows = (h / tile_size).max(1);

        for row in 0..rows {
            for col in 0..cols {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let rotation = (rng >> 33) as usize % 4; // 0, 1, 2, 3 → 0°, 90°, 180°, 270°
                let color = colors[((row + col + rotation) % colors.len())];

                // Draw an L-shape occupying roughly 75% of the tile
                let base_x = col * tile_size;
                let base_y = row * tile_size;
                let arm = (tile_size * 3 / 4).max(2);

                let (lx, ly, lw, lh) = match rotation % 4 {
                    0 => (base_x, base_y, arm, tile_size),
                    1 => (base_x, base_y + tile_size - arm, tile_size, arm),
                    2 => (base_x + tile_size - arm, base_y, arm, tile_size),
                    _ => (base_x, base_y, tile_size, arm),
                };

                for py in ly..((ly + lh).min(h)) {
                    for px in lx..((lx + lw).min(w)) {
                        canvas[py][px] = color;
                    }
                }
            }
        }

        canvas
    }

    // -----------------------------------------------------------------------
    // Tile rasteriser
    // -----------------------------------------------------------------------

    /// Rasterise a set of [`TileShape`] polygons onto a pixel canvas.
    pub fn render_tiles(
        &self,
        tiles: &[TileShape],
        width: u32,
        height: u32,
        bg: [u8; 3],
    ) -> Vec<Vec<[u8; 3]>> {
        let w = width as usize;
        let h = height as usize;
        let mut canvas = vec![vec![bg; w]; h];

        for tile in tiles {
            fill_polygon(&tile.vertices, tile.color, &mut canvas, w, h);
        }
        canvas
    }

    // -----------------------------------------------------------------------
    // Symmetry transform
    // -----------------------------------------------------------------------

    /// Apply wallpaper group symmetry operations to `point`.
    ///
    /// Supported groups: 1 (identity), 2 (180° rotation), 4 (4-fold rotation),
    /// 6 (6-fold rotation).
    pub fn symmetry_transform(&self, point: (f64, f64), group: u8) -> Vec<(f64, f64)> {
        let (x, y) = point;
        match group {
            1 => vec![(x, y)],
            2 => vec![
                (x, y),
                (-x, -y),
            ],
            4 => vec![
                (x, y),
                (-y, x),
                (-x, -y),
                (y, -x),
            ],
            6 => {
                // 60° rotations
                let angles: Vec<f64> = (0..6).map(|k| k as f64 * PI / 3.0).collect();
                angles
                    .iter()
                    .map(|&a| {
                        let cos_a = a.cos();
                        let sin_a = a.sin();
                        (x * cos_a - y * sin_a, x * sin_a + y * cos_a)
                    })
                    .collect()
            }
            _ => vec![(x, y)],
        }
    }
}

impl Default for EscherGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Polygon rasterisation (scan-line fill)
// ---------------------------------------------------------------------------

fn fill_polygon(
    vertices: &[(f64, f64)],
    color: [u8; 3],
    canvas: &mut Vec<Vec<[u8; 3]>>,
    width: usize,
    height: usize,
) {
    if vertices.len() < 3 || width == 0 || height == 0 {
        return;
    }

    let min_y = vertices.iter().map(|v| v.1).fold(f64::INFINITY, f64::min).max(0.0) as usize;
    let max_y = vertices
        .iter()
        .map(|v| v.1)
        .fold(f64::NEG_INFINITY, f64::max)
        .min((height - 1) as f64) as usize;

    for y in min_y..=max_y {
        let yf = y as f64 + 0.5;
        let mut intersections: Vec<f64> = Vec::new();

        let n = vertices.len();
        for i in 0..n {
            let (x0, y0) = vertices[i];
            let (x1, y1) = vertices[(i + 1) % n];
            if (y0 <= yf && y1 > yf) || (y1 <= yf && y0 > yf) {
                let t = (yf - y0) / (y1 - y0);
                intersections.push(x0 + t * (x1 - x0));
            }
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        for pair in intersections.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let x_start = pair[0].max(0.0).round() as usize;
            let x_end = pair[1].min((width - 1) as f64).round() as usize;
            for x in x_start..=x_end {
                canvas[y][x] = color;
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

    fn gen() -> EscherGenerator {
        EscherGenerator::new()
    }

    // --- parallelogram ---

    #[test]
    fn parallelogram_tile_count() {
        let g = gen();
        let colors = [[255u8, 0, 0], [0, 255, 0]];
        let tiles = g.generate_parallelogram_tiling((50.0, 0.0), (0.0, 50.0), 4, 3, &colors);
        assert_eq!(tiles.len(), 12);
    }

    #[test]
    fn parallelogram_each_tile_has_4_vertices() {
        let g = gen();
        let colors = [[100u8, 100, 100]];
        let tiles = g.generate_parallelogram_tiling((30.0, 0.0), (0.0, 30.0), 2, 2, &colors);
        assert!(tiles.iter().all(|t| t.vertices.len() == 4));
    }

    #[test]
    fn parallelogram_empty_colors_returns_empty() {
        let g = gen();
        let tiles = g.generate_parallelogram_tiling((10.0, 0.0), (0.0, 10.0), 5, 5, &[]);
        assert!(tiles.is_empty());
    }

    // --- hexagonal ---

    #[test]
    fn hexagonal_tile_count() {
        let g = gen();
        let colors = [[0u8, 0, 255]];
        let tiles = g.generate_hexagonal_tiling((0.0, 0.0), 30.0, 3, 4, &colors);
        assert_eq!(tiles.len(), 12);
    }

    #[test]
    fn hexagonal_each_tile_has_6_vertices() {
        let g = gen();
        let colors = [[200u8, 200, 200], [100, 100, 100]];
        let tiles = g.generate_hexagonal_tiling((0.0, 0.0), 20.0, 2, 2, &colors);
        assert!(tiles.iter().all(|t| t.vertices.len() == 6));
    }

    #[test]
    fn hexagonal_empty_colors_returns_empty() {
        let g = gen();
        let tiles = g.generate_hexagonal_tiling((0.0, 0.0), 20.0, 3, 3, &[]);
        assert!(tiles.is_empty());
    }

    // --- lizard pattern ---

    #[test]
    fn lizard_pattern_correct_dimensions() {
        let g = gen();
        let canvas = g.generate_lizard_pattern(200, 150, 1.0, 42);
        assert_eq!(canvas.len(), 150);
        assert!(canvas.iter().all(|row| row.len() == 200));
    }

    #[test]
    fn lizard_pattern_has_coloured_pixels() {
        let g = gen();
        let canvas = g.generate_lizard_pattern(100, 100, 1.0, 99);
        let bg: [u8; 3] = [30, 30, 30];
        let non_bg = canvas.iter().flatten().filter(|&&p| p != bg).count();
        assert!(non_bg > 0, "Expected drawn pixels");
    }

    #[test]
    fn lizard_pattern_zero_size() {
        let g = gen();
        let canvas = g.generate_lizard_pattern(0, 0, 1.0, 0);
        assert!(canvas.is_empty());
    }

    // --- render_tiles ---

    #[test]
    fn render_tiles_fills_background() {
        let g = gen();
        let bg = [50u8, 50, 50];
        let canvas = g.render_tiles(&[], 100, 80, bg);
        assert!(canvas.iter().flatten().all(|&p| p == bg));
    }

    #[test]
    fn render_tiles_draws_polygon() {
        let g = gen();
        let tile = TileShape {
            vertices: vec![(10.0, 10.0), (90.0, 10.0), (90.0, 70.0), (10.0, 70.0)],
            color: [255, 0, 0],
        };
        let canvas = g.render_tiles(&[tile], 100, 80, [0, 0, 0]);
        let red_pixels = canvas.iter().flatten().filter(|&&p| p == [255u8, 0, 0]).count();
        assert!(red_pixels > 0);
    }

    // --- symmetry_transform ---

    #[test]
    fn symmetry_group_1_identity() {
        let g = gen();
        let pts = g.symmetry_transform((3.0, 4.0), 1);
        assert_eq!(pts, vec![(3.0, 4.0)]);
    }

    #[test]
    fn symmetry_group_2_two_points() {
        let g = gen();
        let pts = g.symmetry_transform((1.0, 2.0), 2);
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], (1.0, 2.0));
        assert!((pts[1].0 - (-1.0)).abs() < 1e-9);
        assert!((pts[1].1 - (-2.0)).abs() < 1e-9);
    }

    #[test]
    fn symmetry_group_4_four_points() {
        let g = gen();
        let pts = g.symmetry_transform((1.0, 0.0), 4);
        assert_eq!(pts.len(), 4);
        // First is identity
        assert_eq!(pts[0], (1.0, 0.0));
        // Second is 90° rotation: (-y, x) = (0, 1)
        assert!((pts[1].0 - 0.0).abs() < 1e-9);
        assert!((pts[1].1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn symmetry_group_6_six_points() {
        let g = gen();
        let pts = g.symmetry_transform((1.0, 0.0), 6);
        assert_eq!(pts.len(), 6);
        // All points should lie on a unit circle
        for (x, y) in &pts {
            let dist = (x * x + y * y).sqrt();
            assert!((dist - 1.0).abs() < 1e-9, "dist={dist}");
        }
    }

    #[test]
    fn symmetry_unknown_group_returns_identity() {
        let g = gen();
        let pts = g.symmetry_transform((5.0, 7.0), 99);
        assert_eq!(pts, vec![(5.0, 7.0)]);
    }
}
