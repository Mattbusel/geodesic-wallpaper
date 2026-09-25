//! Penrose P3 rhombus tiling — quasiperiodic pattern generator.
//!
//! Implements the substitution / inflation rules for the P3 (thick + thin
//! rhombus) Penrose tiling, starting from a decagonal arrangement of thick
//! rhombuses and iterating inflation rules.

/// Golden ratio φ = (1 + √5) / 2.
pub const PHI: f64 = 1.618_033_988_749_895;

// ── PenroseTileType ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenroseTileType {
    /// Thin rhombus (acute angle 36°).
    KiteDart,
    /// Thin rhombus (36°/144°).
    RhombusThin,
    /// Thick rhombus (72°/108°).
    RhombusThick,
}

// ── PenroseTile ───────────────────────────────────────────────────────────────

/// A single Penrose tile with its geometry and colour.
#[derive(Debug, Clone)]
pub struct PenroseTile {
    pub tile_type: PenroseTileType,
    /// Vertices in order (4 for rhombuses).
    pub vertices: Vec<(f64, f64)>,
    pub color: [u8; 3],
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

fn lerp2(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

fn mid(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    lerp2(a, b, 0.5)
}

/// Divide point between a and b at ratio 1/φ from a.
fn golden_div(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    lerp2(a, b, 1.0 / PHI)
}

fn make_rhombus(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> Vec<(f64, f64)> {
    vec![a, b, c, d]
}

// ── Substitution rules ────────────────────────────────────────────────────────

/// Split a thick rhombus (72°) into 1 thin + 1 thick child.
///
/// Vertices are assumed to be `[v0, v1, v2, v3]` in order (parallelogram).
pub fn subdivide_thick(tile: &PenroseTile) -> Vec<PenroseTile> {
    assert_eq!(tile.tile_type, PenroseTileType::RhombusThick);
    let v = &tile.vertices;
    if v.len() < 4 {
        return vec![tile.clone()];
    }
    let (v0, v1, v2, v3) = (v[0], v[1], v[2], v[3]);

    // P is the golden-ratio division point on the diagonal v0→v2.
    let p = golden_div(v0, v2);
    // Q is the golden-ratio division point on v1→v3.
    let q = golden_div(v1, v3);

    // Thin child.
    let thin = PenroseTile {
        tile_type: PenroseTileType::RhombusThin,
        vertices: make_rhombus(v0, p, q, v1),
        color: [180, 220, 255],
    };
    // Thick child.
    let thick = PenroseTile {
        tile_type: PenroseTileType::RhombusThick,
        vertices: make_rhombus(p, v2, v3, q),
        color: [255, 220, 140],
    };
    vec![thin, thick]
}

/// Split a thin rhombus (36°) into 2 thick + 1 thin child.
///
/// Vertices: `[v0, v1, v2, v3]`.
pub fn subdivide_thin(tile: &PenroseTile) -> Vec<PenroseTile> {
    assert!(
        tile.tile_type == PenroseTileType::RhombusThin
            || tile.tile_type == PenroseTileType::KiteDart
    );
    let v = &tile.vertices;
    if v.len() < 4 {
        return vec![tile.clone()];
    }
    let (v0, v1, v2, v3) = (v[0], v[1], v[2], v[3]);

    // Divide diagonal v0→v2 at 1/φ.
    let p = golden_div(v0, v2);
    // Midpoint of v1→v3.
    let m = mid(v1, v3);

    // Two thick children.
    let thick1 = PenroseTile {
        tile_type: PenroseTileType::RhombusThick,
        vertices: make_rhombus(v0, p, m, v1),
        color: [255, 220, 140],
    };
    let thick2 = PenroseTile {
        tile_type: PenroseTileType::RhombusThick,
        vertices: make_rhombus(v0, v3, m, p),
        color: [255, 220, 140],
    };
    // One thin child.
    let thin = PenroseTile {
        tile_type: PenroseTileType::RhombusThin,
        vertices: make_rhombus(p, v2, v3, m),
        color: [180, 220, 255],
    };
    vec![thick1, thick2, thin]
}

// ── PenroseTiling ─────────────────────────────────────────────────────────────

/// Penrose P3 tiling engine.
pub struct PenroseTiling {
    center: (f64, f64),
    radius: f64,
    initial_tiles: Vec<PenroseTile>,
}

impl PenroseTiling {
    /// Initialise with 10 thick rhombuses arranged with decagonal symmetry.
    pub fn new(center: (f64, f64), radius: f64) -> Self {
        let mut tiles = Vec::with_capacity(10);
        let r = radius;
        let cx = center.0;
        let cy = center.1;

        for k in 0..10 {
            let angle0 = std::f64::consts::PI * (2.0 * k as f64) / 10.0;
            let angle1 = std::f64::consts::PI * (2.0 * (k + 1) as f64) / 10.0;
            let angle_mid = (angle0 + angle1) / 2.0;

            let v0 = (cx, cy);
            let v1 = (cx + r * angle0.cos(), cy + r * angle0.sin());
            let v2 = (
                cx + r * PHI * angle_mid.cos(),
                cy + r * PHI * angle_mid.sin(),
            );
            let v3 = (cx + r * angle1.cos(), cy + r * angle1.sin());

            tiles.push(PenroseTile {
                tile_type: PenroseTileType::RhombusThick,
                vertices: vec![v0, v1, v2, v3],
                color: [255, 220, 140],
            });
        }

        Self {
            center,
            radius,
            initial_tiles: tiles,
        }
    }

    /// Apply N rounds of subdivision and return all resulting tiles.
    pub fn inflate(&self, iterations: usize) -> Vec<PenroseTile> {
        let mut tiles = self.initial_tiles.clone();
        for _ in 0..iterations {
            let mut next = Vec::with_capacity(tiles.len() * 3);
            for tile in &tiles {
                let children = match tile.tile_type {
                    PenroseTileType::RhombusThick => subdivide_thick(tile),
                    PenroseTileType::RhombusThin | PenroseTileType::KiteDart => {
                        subdivide_thin(tile)
                    }
                };
                next.extend(children);
            }
            tiles = next;
        }
        tiles
    }

    /// Rasterise a tile set into an image buffer.
    ///
    /// Returns a `height × width` pixel grid of `[R, G, B]` bytes.
    pub fn render(
        tiles: &[PenroseTile],
        width: u32,
        height: u32,
        bg: [u8; 3],
    ) -> Vec<Vec<[u8; 3]>> {
        let mut image = vec![vec![bg; width as usize]; height as usize];
        let hw = width as f64 / 2.0;
        let hh = height as f64 / 2.0;

        for tile in tiles {
            if tile.vertices.len() < 3 {
                continue;
            }
            // Bounding box in pixel space.
            let px_verts: Vec<(f64, f64)> = tile
                .vertices
                .iter()
                .map(|&(x, y)| (x + hw, hh - y))
                .collect();

            let min_x = px_verts
                .iter()
                .map(|v| v.0)
                .fold(f64::INFINITY, f64::min)
                .floor() as i64;
            let max_x = px_verts
                .iter()
                .map(|v| v.0)
                .fold(f64::NEG_INFINITY, f64::max)
                .ceil() as i64;
            let min_y = px_verts
                .iter()
                .map(|v| v.1)
                .fold(f64::INFINITY, f64::min)
                .floor() as i64;
            let max_y = px_verts
                .iter()
                .map(|v| v.1)
                .fold(f64::NEG_INFINITY, f64::max)
                .ceil() as i64;

            for py in min_y.max(0)..max_y.min(height as i64) {
                for px in min_x.max(0)..max_x.min(width as i64) {
                    if point_in_polygon(px as f64 + 0.5, py as f64 + 0.5, &px_verts) {
                        image[py as usize][px as usize] = tile.color;
                    }
                }
            }
        }
        image
    }
}

/// Point-in-polygon test using the winding-number ray-casting method.
fn point_in_polygon(px: f64, py: f64, verts: &[(f64, f64)]) -> bool {
    let n = verts.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = verts[i];
        let (xj, yj) = verts[j];
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_value() {
        let expected = (1.0 + 5.0f64.sqrt()) / 2.0;
        assert!((PHI - expected).abs() < 1e-10);
    }

    #[test]
    fn initial_tiling_has_10_tiles() {
        let tiling = PenroseTiling::new((0.0, 0.0), 100.0);
        assert_eq!(tiling.initial_tiles.len(), 10);
    }

    #[test]
    fn all_initial_tiles_are_thick() {
        let tiling = PenroseTiling::new((0.0, 0.0), 100.0);
        for tile in &tiling.initial_tiles {
            assert_eq!(tile.tile_type, PenroseTileType::RhombusThick);
        }
    }

    #[test]
    fn inflate_0_returns_10() {
        let tiling = PenroseTiling::new((0.0, 0.0), 100.0);
        let tiles = tiling.inflate(0);
        assert_eq!(tiles.len(), 10);
    }

    #[test]
    fn inflate_1_grows_count() {
        let tiling = PenroseTiling::new((0.0, 0.0), 100.0);
        let tiles = tiling.inflate(1);
        // Each thick → 2 children → at least 20.
        assert!(tiles.len() >= 20);
    }

    #[test]
    fn inflate_2_grows_more() {
        let tiling = PenroseTiling::new((0.0, 0.0), 100.0);
        let t1 = tiling.inflate(1).len();
        let t2 = tiling.inflate(2).len();
        assert!(t2 > t1);
    }

    #[test]
    fn subdivide_thick_produces_two_tiles() {
        let tile = PenroseTile {
            tile_type: PenroseTileType::RhombusThick,
            vertices: vec![(0.0, 0.0), (1.0, 0.0), (1.5, 0.5), (0.5, 0.5)],
            color: [255, 220, 140],
        };
        let children = subdivide_thick(&tile);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].tile_type, PenroseTileType::RhombusThin);
        assert_eq!(children[1].tile_type, PenroseTileType::RhombusThick);
    }

    #[test]
    fn subdivide_thin_produces_three_tiles() {
        let tile = PenroseTile {
            tile_type: PenroseTileType::RhombusThin,
            vertices: vec![(0.0, 0.0), (0.5, 0.1), (1.5, 0.3), (1.0, 0.2)],
            color: [180, 220, 255],
        };
        let children = subdivide_thin(&tile);
        assert_eq!(children.len(), 3);
        let thick_count = children
            .iter()
            .filter(|t| t.tile_type == PenroseTileType::RhombusThick)
            .count();
        let thin_count = children
            .iter()
            .filter(|t| t.tile_type == PenroseTileType::RhombusThin)
            .count();
        assert_eq!(thick_count, 2);
        assert_eq!(thin_count, 1);
    }

    #[test]
    fn render_returns_correct_dimensions() {
        let tiling = PenroseTiling::new((0.0, 0.0), 50.0);
        let tiles = tiling.inflate(1);
        let img = PenroseTiling::render(&tiles, 64, 48, [0, 0, 0]);
        assert_eq!(img.len(), 48);
        assert_eq!(img[0].len(), 64);
    }

    #[test]
    fn render_paints_some_pixels() {
        let tiling = PenroseTiling::new((0.0, 0.0), 50.0);
        let tiles = tiling.inflate(1);
        let bg = [0u8, 0, 0];
        let img = PenroseTiling::render(&tiles, 128, 128, bg);
        let painted = img.iter().flatten().any(|&px| px != bg);
        assert!(painted, "render produced a fully black image");
    }

    #[test]
    fn point_in_polygon_inside() {
        let sq = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_polygon(5.0, 5.0, &sq));
    }

    #[test]
    fn point_in_polygon_outside() {
        let sq = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(!point_in_polygon(15.0, 5.0, &sq));
    }
}
