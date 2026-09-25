//! Islamic geometric pattern generator.

use std::f64::consts::PI;

// ---------------------------------------------------------------------------
// PatternType
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PatternType {
    Star { points: u8 },
    Hexagonal,
    Octagonal,
    Twelve,
    Girih,
}

impl PatternType {
    pub fn default_grid_unit(&self) -> f64 {
        match self {
            PatternType::Star { points } => 40.0 + (*points as f64 * 2.0),
            PatternType::Hexagonal => 60.0,
            PatternType::Octagonal => 70.0,
            PatternType::Twelve => 80.0,
            PatternType::Girih => 75.0,
        }
    }
}

// ---------------------------------------------------------------------------
// PatternTile
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PatternTile {
    pub vertices: Vec<(f64, f64)>,
    pub fill_color: [u8; 3],
    pub outline: bool,
}

impl PatternTile {
    pub fn new(vertices: Vec<(f64, f64)>, fill_color: [u8; 3], outline: bool) -> Self {
        Self {
            vertices,
            fill_color,
            outline,
        }
    }
}

// ---------------------------------------------------------------------------
// IslamicPatternGenerator
// ---------------------------------------------------------------------------

pub struct IslamicPatternGenerator;

impl IslamicPatternGenerator {
    /// Generate star polygon vertices.
    /// `inner_ratio` controls how sharp the points are (0 < inner_ratio < 1).
    pub fn generate_star_polygon(
        center: (f64, f64),
        radius: f64,
        points: u8,
        inner_ratio: f64,
    ) -> Vec<(f64, f64)> {
        let n = points as usize;
        if n == 0 {
            return Vec::new();
        }
        let inner_radius = radius * inner_ratio.clamp(0.1, 0.99);
        let mut verts = Vec::with_capacity(n * 2);
        for i in 0..n {
            // Outer vertex
            let outer_angle = 2.0 * PI * i as f64 / n as f64 - PI / 2.0;
            verts.push((
                center.0 + radius * outer_angle.cos(),
                center.1 + radius * outer_angle.sin(),
            ));
            // Inner vertex (halfway between adjacent outer vertices)
            let inner_angle = outer_angle + PI / n as f64;
            verts.push((
                center.0 + inner_radius * inner_angle.cos(),
                center.1 + inner_radius * inner_angle.sin(),
            ));
        }
        verts
    }

    /// Tile a pattern across an image of given dimensions.
    pub fn tile_plane(
        pattern: PatternType,
        width: u32,
        height: u32,
        unit: f64,
    ) -> Vec<PatternTile> {
        let mut tiles = Vec::new();
        let cols = (width as f64 / unit).ceil() as i32 + 1;
        let rows = (height as f64 / unit).ceil() as i32 + 1;

        // Color palette for alternating tiles
        let colors: [[u8; 3]; 3] = [
            [30, 80, 150],   // deep blue
            [200, 160, 40],  // gold
            [220, 220, 220], // light grey
        ];

        let mut color_idx = 0usize;

        for row in 0..rows {
            for col in 0..cols {
                let cx = col as f64 * unit + unit * 0.5;
                let cy = row as f64 * unit + unit * 0.5;
                let center = (cx, cy);
                let color = colors[color_idx % colors.len()];
                color_idx += 1;

                let verts = match &pattern {
                    PatternType::Star { points } => {
                        Self::generate_star_polygon(center, unit * 0.45, *points, 0.45)
                    }
                    PatternType::Hexagonal => {
                        Self::generate_regular_polygon(center, unit * 0.48, 6)
                    }
                    PatternType::Octagonal => {
                        Self::generate_regular_polygon(center, unit * 0.45, 8)
                    }
                    PatternType::Twelve => Self::generate_regular_polygon(center, unit * 0.45, 12),
                    PatternType::Girih => Self::generate_girih_pentagon(center, unit * 0.45),
                };

                tiles.push(PatternTile::new(verts, color, true));
            }
        }

        tiles
    }

    /// Simple scanline rasterizer for a set of tiles.
    /// Returns a width×height pixel grid (row-major).
    pub fn render(
        tiles: &[PatternTile],
        width: u32,
        height: u32,
        bg_color: [u8; 3],
    ) -> Vec<Vec<[u8; 3]>> {
        let w = width as usize;
        let h = height as usize;
        let mut image: Vec<Vec<[u8; 3]>> = vec![vec![bg_color; w]; h];

        for tile in tiles {
            if tile.vertices.len() < 3 {
                continue;
            }
            // Compute bounding box
            let (min_x, min_y, max_x, max_y) = bounding_box(&tile.vertices);
            let y0 = (min_y.floor() as i32).max(0) as usize;
            let y1 = (max_y.ceil() as i32).min(h as i32 - 1) as usize;
            let x0 = (min_x.floor() as i32).max(0) as usize;
            let x1 = (max_x.ceil() as i32).min(w as i32 - 1) as usize;

            for py in y0..=y1 {
                for px in x0..=x1 {
                    if point_in_polygon(px as f64 + 0.5, py as f64 + 0.5, &tile.vertices) {
                        image[py][px] = tile.fill_color;
                    }
                }
            }
        }

        image
    }

    /// Alternate weaving colors for an over/under interlace effect.
    pub fn interlace_pattern(tiles: &[PatternTile]) -> Vec<PatternTile> {
        tiles
            .iter()
            .enumerate()
            .map(|(i, tile)| {
                let color = if i % 2 == 0 {
                    // "over" strand: slightly lighter
                    blend_color(tile.fill_color, [255, 255, 255], 0.3)
                } else {
                    // "under" strand: slightly darker
                    blend_color(tile.fill_color, [0, 0, 0], 0.3)
                };
                PatternTile::new(tile.vertices.clone(), color, tile.outline)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn generate_regular_polygon(center: (f64, f64), radius: f64, n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let angle = 2.0 * PI * i as f64 / n as f64 - PI / 2.0;
            (
                center.0 + radius * angle.cos(),
                center.1 + radius * angle.sin(),
            )
        })
        .collect()
}

impl IslamicPatternGenerator {
    fn generate_regular_polygon(center: (f64, f64), radius: f64, n: usize) -> Vec<(f64, f64)> {
        generate_regular_polygon(center, radius, n)
    }

    fn generate_girih_pentagon(center: (f64, f64), radius: f64) -> Vec<(f64, f64)> {
        // Girih "decagon" tile approximated as an irregular pentagon
        let angles = [90.0_f64, 162.0, 234.0, 306.0, 18.0];
        angles
            .iter()
            .map(|&deg| {
                let rad = deg.to_radians();
                (center.0 + radius * rad.cos(), center.1 + radius * rad.sin())
            })
            .collect()
    }
}

fn bounding_box(verts: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    let min_x = verts.iter().map(|v| v.0).fold(f64::INFINITY, f64::min);
    let min_y = verts.iter().map(|v| v.1).fold(f64::INFINITY, f64::min);
    let max_x = verts.iter().map(|v| v.0).fold(f64::NEG_INFINITY, f64::max);
    let max_y = verts.iter().map(|v| v.1).fold(f64::NEG_INFINITY, f64::max);
    (min_x, min_y, max_x, max_y)
}

/// Point-in-polygon test (ray casting).
fn point_in_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    let n = polygon.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn blend_color(base: [u8; 3], target: [u8; 3], factor: f64) -> [u8; 3] {
    let f = factor.clamp(0.0, 1.0);
    [
        ((base[0] as f64 * (1.0 - f) + target[0] as f64 * f) as u8),
        ((base[1] as f64 * (1.0 - f) + target[1] as f64 * f) as u8),
        ((base[2] as f64 * (1.0 - f) + target[2] as f64 * f) as u8),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_star_polygon_vertex_count() {
        let verts = IslamicPatternGenerator::generate_star_polygon((0.0, 0.0), 50.0, 8, 0.45);
        assert_eq!(verts.len(), 16); // 8 outer + 8 inner
    }

    #[test]
    fn test_star_polygon_6_points() {
        let verts = IslamicPatternGenerator::generate_star_polygon((0.0, 0.0), 50.0, 6, 0.4);
        assert_eq!(verts.len(), 12);
    }

    #[test]
    fn test_star_polygon_zero_points() {
        let verts = IslamicPatternGenerator::generate_star_polygon((0.0, 0.0), 50.0, 0, 0.45);
        assert!(verts.is_empty());
    }

    #[test]
    fn test_tile_plane_nonempty() {
        let tiles = IslamicPatternGenerator::tile_plane(PatternType::Hexagonal, 200, 200, 60.0);
        assert!(!tiles.is_empty());
    }

    #[test]
    fn test_tile_plane_star() {
        let tiles =
            IslamicPatternGenerator::tile_plane(PatternType::Star { points: 8 }, 400, 400, 80.0);
        assert!(!tiles.is_empty());
        for tile in &tiles {
            // Each star tile should have 16 vertices (8 points)
            assert_eq!(tile.vertices.len(), 16);
        }
    }

    #[test]
    fn test_tile_plane_octagonal() {
        let tiles = IslamicPatternGenerator::tile_plane(PatternType::Octagonal, 200, 200, 70.0);
        for tile in &tiles {
            assert_eq!(tile.vertices.len(), 8);
        }
    }

    #[test]
    fn test_render_dimensions() {
        let tiles = IslamicPatternGenerator::tile_plane(PatternType::Hexagonal, 50, 50, 25.0);
        let img = IslamicPatternGenerator::render(&tiles, 50, 50, [0, 0, 0]);
        assert_eq!(img.len(), 50);
        assert_eq!(img[0].len(), 50);
    }

    #[test]
    fn test_render_has_color() {
        let tiles = IslamicPatternGenerator::tile_plane(PatternType::Hexagonal, 100, 100, 40.0);
        let img = IslamicPatternGenerator::render(&tiles, 100, 100, [0, 0, 0]);
        let has_nonblack = img.iter().flatten().any(|&p| p != [0u8, 0, 0]);
        assert!(has_nonblack, "Render should produce colored pixels");
    }

    #[test]
    fn test_interlace_pattern_length() {
        let tiles = IslamicPatternGenerator::tile_plane(PatternType::Hexagonal, 100, 100, 40.0);
        let interlaced = IslamicPatternGenerator::interlace_pattern(&tiles);
        assert_eq!(interlaced.len(), tiles.len());
    }

    #[test]
    fn test_interlace_colors_differ() {
        let tiles = IslamicPatternGenerator::tile_plane(PatternType::Hexagonal, 100, 100, 40.0);
        if tiles.len() >= 2 {
            let interlaced = IslamicPatternGenerator::interlace_pattern(&tiles);
            // Even and odd tiles should have different colors due to blend
            let c0 = interlaced[0].fill_color;
            let c1 = interlaced[1].fill_color;
            assert_ne!(c0, c1, "Interlaced tiles should alternate colors");
        }
    }

    #[test]
    fn test_default_grid_units() {
        assert!(PatternType::Hexagonal.default_grid_unit() > 0.0);
        assert!(PatternType::Star { points: 8 }.default_grid_unit() > 0.0);
        assert!(PatternType::Girih.default_grid_unit() > 0.0);
    }

    #[test]
    fn test_point_in_polygon_inside() {
        // Square from (0,0) to (10,10)
        let sq = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_polygon(5.0, 5.0, &sq));
    }

    #[test]
    fn test_point_in_polygon_outside() {
        let sq = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(!point_in_polygon(15.0, 15.0, &sq));
    }
}
