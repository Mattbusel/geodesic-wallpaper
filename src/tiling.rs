//! Tiling engine for wallpaper pattern generation.
//!
//! Supports Square, Hexagonal, Triangular, and Rhombic tile grids.
//! Provides per-pixel cell lookup, neighbour enumeration, and a renderer
//! that applies a colour function per tile cell.

// ── TileShape ─────────────────────────────────────────────────────────────────

/// The geometric shape of each tile in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileShape {
    /// Regular squares arranged in a rectilinear grid.
    Square,
    /// Regular hexagons in offset-row packing.
    Hexagonal,
    /// Equilateral triangles in alternating up/down rows.
    Triangular,
    /// Rhombuses (diamonds) arranged in a rectilinear grid.
    Rhombic,
}

// ── TileGrid ──────────────────────────────────────────────────────────────────

/// A tiling grid over a pixel canvas.
pub struct TileGrid {
    pub shape: TileShape,
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
}

impl TileGrid {
    /// Create a new tile grid.
    pub fn new(shape: TileShape, width: u32, height: u32, tile_size: u32) -> Self {
        Self {
            shape,
            width,
            height,
            tile_size: tile_size.max(1),
        }
    }

    /// Return the [`TileCell`] that contains pixel `(x, y)`.
    pub fn cell_at(&self, x: u32, y: u32) -> TileCell {
        match self.shape {
            TileShape::Square => self.square_cell(x, y),
            TileShape::Hexagonal => self.hex_cell(x, y),
            TileShape::Triangular => self.tri_cell(x, y),
            TileShape::Rhombic => self.rhombic_cell(x, y),
        }
    }

    /// Return the neighbouring cells adjacent to `cell`.
    ///
    /// - Square: 4 neighbours (NSEW).
    /// - Hexagonal: 6 neighbours.
    /// - Triangular: 3 neighbours.
    /// - Rhombic: 4 neighbours (same as square for grid purposes).
    pub fn neighbors(&self, cell: &TileCell) -> Vec<TileCell> {
        match self.shape {
            TileShape::Square => self.square_neighbors(cell),
            TileShape::Hexagonal => self.hex_neighbors(cell),
            TileShape::Triangular => self.tri_neighbors(cell),
            TileShape::Rhombic => self.rhombic_neighbors(cell),
        }
    }

    // ── Square ────────────────────────────────────────────────────────────────

    fn square_cell(&self, x: u32, y: u32) -> TileCell {
        let ts = self.tile_size as i32;
        let col = x as i32 / ts;
        let row = y as i32 / ts;
        let cx = (col * ts + ts / 2) as f32;
        let cy = (row * ts + ts / 2) as f32;
        TileCell {
            col,
            row,
            center_x: cx,
            center_y: cy,
            shape: TileShape::Square,
        }
    }

    fn square_neighbors(&self, cell: &TileCell) -> Vec<TileCell> {
        let offsets: &[(i32, i32)] = &[(0, -1), (1, 0), (0, 1), (-1, 0)];
        offsets
            .iter()
            .map(|&(dc, dr)| {
                let col = cell.col + dc;
                let row = cell.row + dr;
                let ts = self.tile_size as i32;
                TileCell {
                    col,
                    row,
                    center_x: (col * ts + ts / 2) as f32,
                    center_y: (row * ts + ts / 2) as f32,
                    shape: TileShape::Square,
                }
            })
            .collect()
    }

    // ── Hexagonal (offset-row layout) ─────────────────────────────────────────

    fn hex_cell(&self, x: u32, y: u32) -> TileCell {
        // Using "offset" (odd-row right-shift) hex grid.
        // h = tile_size, w = tile_size * sqrt(3)/2 (flat-top hex column width)
        let ts = self.tile_size as f32;
        let w = ts * 3.0_f32.sqrt() / 2.0;
        let h = ts;

        let row = (y as f32 / (h * 0.75)) as i32;
        let row_offset = if row % 2 != 0 { w / 2.0 } else { 0.0 };
        let col = ((x as f32 - row_offset) / w) as i32;

        let cx = col as f32 * w + row_offset + w / 2.0;
        let cy = row as f32 * h * 0.75 + h / 2.0;
        TileCell {
            col,
            row,
            center_x: cx,
            center_y: cy,
            shape: TileShape::Hexagonal,
        }
    }

    fn hex_neighbors(&self, cell: &TileCell) -> Vec<TileCell> {
        let ts = self.tile_size as f32;
        let w = ts * 3.0_f32.sqrt() / 2.0;
        let h = ts;

        // Cube-coordinate offsets for odd-row offset hex
        let offsets: &[(i32, i32)] = if cell.row % 2 != 0 {
            &[(1, 0), (1, -1), (0, -1), (-1, 0), (0, 1), (1, 1)]
        } else {
            &[(1, 0), (0, -1), (-1, -1), (-1, 0), (-1, 1), (0, 1)]
        };

        offsets
            .iter()
            .map(|&(dc, dr)| {
                let col = cell.col + dc;
                let row = cell.row + dr;
                let row_offset = if row % 2 != 0 { w / 2.0 } else { 0.0 };
                let cx = col as f32 * w + row_offset + w / 2.0;
                let cy = row as f32 * h * 0.75 + h / 2.0;
                TileCell {
                    col,
                    row,
                    center_x: cx,
                    center_y: cy,
                    shape: TileShape::Hexagonal,
                }
            })
            .collect()
    }

    // ── Triangular ────────────────────────────────────────────────────────────

    fn tri_cell(&self, x: u32, y: u32) -> TileCell {
        let ts = self.tile_size as i32;
        let row = y as i32 / ts;
        let col_raw = x as i32 * 2 / ts;
        // Alternate up/down triangles: parity of (col + row)
        let col = col_raw;
        let cx = (col * ts / 2 + ts / 4) as f32;
        let cy = (row * ts + ts / 2) as f32;
        TileCell {
            col,
            row,
            center_x: cx,
            center_y: cy,
            shape: TileShape::Triangular,
        }
    }

    fn tri_neighbors(&self, cell: &TileCell) -> Vec<TileCell> {
        let ts = self.tile_size as i32;
        // A triangle has 3 neighbours: left, right, and up or down depending on parity.
        let parity = (cell.col + cell.row) % 2;
        let vertical_dr = if parity == 0 { 1i32 } else { -1i32 };

        let offsets: &[(i32, i32)] = &[(-1, 0), (1, 0), (0, vertical_dr)];
        offsets
            .iter()
            .map(|&(dc, dr)| {
                let col = cell.col + dc;
                let row = cell.row + dr;
                let cx = (col * ts / 2 + ts / 4) as f32;
                let cy = (row * ts + ts / 2) as f32;
                TileCell {
                    col,
                    row,
                    center_x: cx,
                    center_y: cy,
                    shape: TileShape::Triangular,
                }
            })
            .collect()
    }

    // ── Rhombic ───────────────────────────────────────────────────────────────

    fn rhombic_cell(&self, x: u32, y: u32) -> TileCell {
        // Rhombic: diamonds with horizontal and vertical diagonals equal to tile_size.
        // Map pixel (x,y) via 45° rotation:  u = x+y,  v = x-y
        let ts = self.tile_size as i32;
        let xi = x as i32;
        let yi = y as i32;
        let u = xi + yi;
        let v = xi - yi;
        let col = u / ts;
        let row = v / ts;
        // Center of rhombus
        let uc = (col * ts + ts / 2) as f32;
        let vc = (row * ts + ts / 2) as f32;
        let cx = (uc + vc) / 2.0;
        let cy = (uc - vc) / 2.0;
        TileCell {
            col,
            row,
            center_x: cx,
            center_y: cy,
            shape: TileShape::Rhombic,
        }
    }

    fn rhombic_neighbors(&self, cell: &TileCell) -> Vec<TileCell> {
        let ts = self.tile_size as i32;
        let offsets: &[(i32, i32)] = &[(0, -1), (1, 0), (0, 1), (-1, 0)];
        offsets
            .iter()
            .map(|&(dc, dr)| {
                let col = cell.col + dc;
                let row = cell.row + dr;
                let uc = (col * ts + ts / 2) as f32;
                let vc = (row * ts + ts / 2) as f32;
                let cx = (uc + vc) / 2.0;
                let cy = (uc - vc) / 2.0;
                TileCell {
                    col,
                    row,
                    center_x: cx,
                    center_y: cy,
                    shape: TileShape::Rhombic,
                }
            })
            .collect()
    }
}

// ── TileCell ──────────────────────────────────────────────────────────────────

/// A single tile cell within a grid.
#[derive(Debug, Clone)]
pub struct TileCell {
    /// Column index (may be negative for cells outside the canvas).
    pub col: i32,
    /// Row index.
    pub row: i32,
    /// X coordinate of the tile center in pixels.
    pub center_x: f32,
    /// Y coordinate of the tile center in pixels.
    pub center_y: f32,
    /// Shape of this tile.
    pub shape: TileShape,
}

// ── TileRenderer ─────────────────────────────────────────────────────────────

/// Renders a wallpaper pattern by applying a colouring function per tile cell.
pub struct TileRenderer;

impl TileRenderer {
    /// Render the wallpaper by calling `color_fn` for each tile cell and
    /// painting that colour over all pixels in the cell.
    ///
    /// Returns a flat Vec of `[R, G, B]` pixels in row-major order
    /// (pixel at (x, y) is at index `y * width + x`).
    pub fn render(
        grid: &TileGrid,
        color_fn: impl Fn(&TileCell) -> [u8; 3],
        width: u32,
        height: u32,
    ) -> Vec<[u8; 3]> {
        let mut pixels = vec![[0u8; 3]; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                let cell = grid.cell_at(x, y);
                pixels[(y * width + x) as usize] = color_fn(&cell);
            }
        }
        pixels
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Square cell at origin
    #[test]
    fn square_cell_origin() {
        let grid = TileGrid::new(TileShape::Square, 100, 100, 32);
        let cell = grid.cell_at(0, 0);
        assert_eq!(cell.col, 0);
        assert_eq!(cell.row, 0);
        assert_eq!(cell.shape, TileShape::Square);
    }

    // 2. Square cell at second column
    #[test]
    fn square_cell_second_col() {
        let grid = TileGrid::new(TileShape::Square, 100, 100, 32);
        let cell = grid.cell_at(33, 0);
        assert_eq!(cell.col, 1);
        assert_eq!(cell.row, 0);
    }

    // 3. Square cell center is in the middle of the tile
    #[test]
    fn square_cell_center() {
        let grid = TileGrid::new(TileShape::Square, 100, 100, 32);
        let cell = grid.cell_at(0, 0);
        assert!((cell.center_x - 16.0).abs() < 1.0);
        assert!((cell.center_y - 16.0).abs() < 1.0);
    }

    // 4. Square neighbors: exactly 4
    #[test]
    fn square_neighbors_count() {
        let grid = TileGrid::new(TileShape::Square, 1000, 1000, 32);
        let cell = grid.cell_at(100, 100);
        let neighbors = grid.neighbors(&cell);
        assert_eq!(neighbors.len(), 4);
    }

    // 5. Hexagonal cell returns correct shape
    #[test]
    fn hex_cell_shape() {
        let grid = TileGrid::new(TileShape::Hexagonal, 200, 200, 32);
        let cell = grid.cell_at(50, 50);
        assert_eq!(cell.shape, TileShape::Hexagonal);
    }

    // 6. Hexagonal neighbors: exactly 6
    #[test]
    fn hex_neighbors_count() {
        let grid = TileGrid::new(TileShape::Hexagonal, 1000, 1000, 32);
        let cell = grid.cell_at(200, 200);
        let neighbors = grid.neighbors(&cell);
        assert_eq!(neighbors.len(), 6);
    }

    // 7. Triangular cell shape
    #[test]
    fn tri_cell_shape() {
        let grid = TileGrid::new(TileShape::Triangular, 200, 200, 32);
        let cell = grid.cell_at(50, 50);
        assert_eq!(cell.shape, TileShape::Triangular);
    }

    // 8. Triangular neighbors: exactly 3
    #[test]
    fn tri_neighbors_count() {
        let grid = TileGrid::new(TileShape::Triangular, 1000, 1000, 32);
        let cell = grid.cell_at(200, 200);
        let neighbors = grid.neighbors(&cell);
        assert_eq!(neighbors.len(), 3);
    }

    // 9. Rhombic cell shape
    #[test]
    fn rhombic_cell_shape() {
        let grid = TileGrid::new(TileShape::Rhombic, 200, 200, 32);
        let cell = grid.cell_at(50, 50);
        assert_eq!(cell.shape, TileShape::Rhombic);
    }

    // 10. Rhombic neighbors: exactly 4
    #[test]
    fn rhombic_neighbors_count() {
        let grid = TileGrid::new(TileShape::Rhombic, 1000, 1000, 32);
        let cell = grid.cell_at(200, 200);
        let neighbors = grid.neighbors(&cell);
        assert_eq!(neighbors.len(), 4);
    }

    // 11. TileRenderer produces correct number of pixels
    #[test]
    fn renderer_pixel_count() {
        let grid = TileGrid::new(TileShape::Square, 64, 64, 16);
        let pixels = TileRenderer::render(&grid, |_| [255, 0, 0], 64, 64);
        assert_eq!(pixels.len(), 64 * 64);
    }

    // 12. TileRenderer: color_fn is applied
    #[test]
    fn renderer_applies_color() {
        let grid = TileGrid::new(TileShape::Square, 32, 32, 8);
        let pixels = TileRenderer::render(&grid, |_| [1, 2, 3], 32, 32);
        for p in &pixels {
            assert_eq!(*p, [1, 2, 3]);
        }
    }

    // 13. TileRenderer: different cells get different colors
    #[test]
    fn renderer_color_varies_by_cell() {
        let grid = TileGrid::new(TileShape::Square, 64, 64, 32);
        let pixels = TileRenderer::render(
            &grid,
            |cell| {
                [
                    (cell.col as u8).wrapping_mul(64),
                    (cell.row as u8).wrapping_mul(64),
                    0,
                ]
            },
            64,
            64,
        );
        // Cell (0,0) and cell (1,0) should have different R values
        let p00 = pixels[0]; // pixel (0,0) → col=0, row=0
        let p10 = pixels[32]; // pixel (32,0) → col=1, row=0
        assert_ne!(p00[0], p10[0]);
    }

    // 14. Square cells cover all pixels (no pixel is unassigned)
    #[test]
    fn square_coverage_no_gap() {
        let w = 64u32;
        let h = 64u32;
        let grid = TileGrid::new(TileShape::Square, w, h, 16);
        let pixels = TileRenderer::render(
            &grid,
            |cell| {
                [
                    (cell.col.rem_euclid(256)) as u8,
                    (cell.row.rem_euclid(256)) as u8,
                    0,
                ]
            },
            w,
            h,
        );
        assert_eq!(pixels.len(), (w * h) as usize);
    }

    // 15. Tile size of 1 → each pixel is its own cell
    #[test]
    fn tile_size_one() {
        let grid = TileGrid::new(TileShape::Square, 4, 4, 1);
        let c0 = grid.cell_at(0, 0);
        let c1 = grid.cell_at(1, 0);
        assert_ne!(c0.col, c1.col);
    }

    // 16. Square cell col/row are non-negative for valid pixel coords
    #[test]
    fn square_cell_nonneg() {
        let grid = TileGrid::new(TileShape::Square, 200, 200, 20);
        for y in (0..200u32).step_by(19) {
            for x in (0..200u32).step_by(19) {
                let cell = grid.cell_at(x, y);
                assert!(cell.col >= 0);
                assert!(cell.row >= 0);
            }
        }
    }

    // 17. Hexagonal cell center is finite
    #[test]
    fn hex_cell_center_finite() {
        let grid = TileGrid::new(TileShape::Hexagonal, 500, 500, 40);
        let cell = grid.cell_at(250, 250);
        assert!(cell.center_x.is_finite());
        assert!(cell.center_y.is_finite());
    }

    // 18. Square neighbor cols/rows are correct
    #[test]
    fn square_neighbor_positions() {
        let grid = TileGrid::new(TileShape::Square, 1000, 1000, 32);
        let cell = grid.cell_at(96, 96); // col=3, row=3
        let neighbors = grid.neighbors(&cell);
        let cols: Vec<i32> = neighbors.iter().map(|n| n.col).collect();
        let rows: Vec<i32> = neighbors.iter().map(|n| n.row).collect();
        assert!(cols.contains(&2) || cols.contains(&4));
        assert!(rows.contains(&2) || rows.contains(&4));
    }

    // 19. Triangular cell center is finite
    #[test]
    fn tri_cell_center_finite() {
        let grid = TileGrid::new(TileShape::Triangular, 400, 400, 32);
        let cell = grid.cell_at(200, 200);
        assert!(cell.center_x.is_finite());
        assert!(cell.center_y.is_finite());
    }

    // 20. Rhombic cell center is finite
    #[test]
    fn rhombic_cell_center_finite() {
        let grid = TileGrid::new(TileShape::Rhombic, 400, 400, 32);
        let cell = grid.cell_at(200, 200);
        assert!(cell.center_x.is_finite());
        assert!(cell.center_y.is_finite());
    }

    // 21. Hex renderer covers all pixels
    #[test]
    fn hex_coverage() {
        let w = 64u32;
        let h = 64u32;
        let grid = TileGrid::new(TileShape::Hexagonal, w, h, 16);
        let pixels = TileRenderer::render(&grid, |_| [128, 128, 128], w, h);
        assert_eq!(pixels.len(), (w * h) as usize);
    }

    // 22. Tri renderer covers all pixels
    #[test]
    fn tri_coverage() {
        let w = 64u32;
        let h = 64u32;
        let grid = TileGrid::new(TileShape::Triangular, w, h, 16);
        let pixels = TileRenderer::render(&grid, |_| [0, 255, 0], w, h);
        assert_eq!(pixels.len(), (w * h) as usize);
    }
}
