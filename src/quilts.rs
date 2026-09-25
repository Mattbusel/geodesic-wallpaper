//! Quilting pattern generator.

// ---------------------------------------------------------------------------
// QuiltBlock
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum QuiltBlock {
    NinePatches,
    PinwheelBlock,
    StarBlock,
    BasketWeave,
    LogCabin,
}

impl QuiltBlock {
    /// Canonical pixel size of one block.
    pub fn block_size(&self) -> u32 {
        match self {
            QuiltBlock::NinePatches => 90,
            QuiltBlock::PinwheelBlock => 80,
            QuiltBlock::StarBlock => 96,
            QuiltBlock::BasketWeave => 80,
            QuiltBlock::LogCabin => 100,
        }
    }
}

// ---------------------------------------------------------------------------
// ColorPalette
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ColorPalette {
    pub colors: Vec<[u8; 3]>,
}

impl ColorPalette {
    pub fn new(colors: Vec<[u8; 3]>) -> Self {
        Self { colors }
    }

    /// Get color at index, wrapping around.
    pub fn get(&self, index: usize) -> [u8; 3] {
        if self.colors.is_empty() {
            return [128, 128, 128];
        }
        self.colors[index % self.colors.len()]
    }

    /// Simple default palette with 6 colors.
    pub fn default_palette() -> Self {
        Self::new(vec![
            [220, 60, 60],   // red
            [60, 120, 220],  // blue
            [60, 180, 60],   // green
            [220, 180, 60],  // gold
            [180, 60, 180],  // purple
            [240, 240, 240], // white
        ])
    }
}

// ---------------------------------------------------------------------------
// Internal LCG RNG
// ---------------------------------------------------------------------------

struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1))
    }
    fn next_usize(&mut self, n: usize) -> usize {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^= z >> 31;
        (z as usize) % n.max(1)
    }
}

// ---------------------------------------------------------------------------
// QuiltPattern — block generators
// ---------------------------------------------------------------------------

pub struct QuiltPattern;

impl QuiltPattern {
    /// 3×3 grid of colored squares; colors chosen randomly from palette.
    pub fn generate_nine_patch(palette: &ColorPalette, seed: u64) -> Vec<Vec<[u8; 3]>> {
        let size = 90usize;
        let cell = size / 3;
        let mut rng = Lcg::new(seed);
        let mut grid: Vec<Vec<[u8; 3]>> = vec![vec![[0u8; 3]; size]; size];
        for row in 0..3usize {
            for col in 0..3usize {
                let color = palette.get(rng.next_usize(palette.colors.len().max(1)));
                for py in (row * cell)..((row + 1) * cell).min(size) {
                    for px in (col * cell)..((col + 1) * cell).min(size) {
                        grid[py][px] = color;
                    }
                }
            }
        }
        grid
    }

    /// Pinwheel: four right-triangle quadrants, alternating two colors, rotated.
    pub fn generate_pinwheel(palette: &ColorPalette) -> Vec<Vec<[u8; 3]>> {
        let size = 80usize;
        let half = size / 2;
        let c0 = palette.get(0);
        let c1 = palette.get(1);
        let mut grid: Vec<Vec<[u8; 3]>> = vec![vec![[0u8; 3]; size]; size];

        for py in 0..size {
            for px in 0..size {
                // Determine quadrant and triangle within quadrant for pinwheel effect
                let qx = px as i32 - half as i32;
                let qy = py as i32 - half as i32;
                // Use the sign of qx*qy and abs comparison for pinwheel rotation
                let color = if (qx >= 0 && qy >= 0 && qx >= qy)
                    || (qx < 0 && qy >= 0 && (-qx) < qy)
                    || (qx < 0 && qy < 0 && qx <= qy)
                    || (qx >= 0 && qy < 0 && qx < (-qy))
                {
                    c0
                } else {
                    c1
                };
                grid[py][px] = color;
            }
        }
        grid
    }

    /// Log Cabin: concentric rectangular strips, alternating two color groups.
    pub fn generate_log_cabin(palette: &ColorPalette, rings: u8) -> Vec<Vec<[u8; 3]>> {
        let rings = rings.max(1) as usize;
        // Center cell + rings around it
        let size = 2 * rings + 1;
        let pixel_size = 100usize;
        let cell_size = pixel_size / size.max(1);
        let actual_size = cell_size * size;

        let mut grid: Vec<Vec<[u8; 3]>> = vec![vec![[0u8; 3]; actual_size]; actual_size];

        for py in 0..actual_size {
            for px in 0..actual_size {
                // Distance from edge (ring index)
                let cx = px / cell_size.max(1);
                let cy = py / cell_size.max(1);
                let ring = cx.min(cy).min(size - 1 - cx).min(size - 1 - cy);
                // Alternate dark/light sides (classic log cabin: right+bottom = dark, left+top = light)
                let is_right_or_bottom = {
                    let dcx = cx as i32 - (size / 2) as i32;
                    let dcy = cy as i32 - (size / 2) as i32;
                    dcx.abs() >= dcy.abs() && dcx >= 0 || dcy > dcx.abs() as i32
                };
                let color_idx = if is_right_or_bottom {
                    ring * 2 + 1
                } else {
                    ring * 2
                };
                grid[py][px] = palette.get(color_idx);
            }
        }
        grid
    }

    /// Assemble a list of blocks (each a 2D pixel grid) into a quilt grid.
    pub fn assemble_quilt(blocks: &[Vec<Vec<[u8; 3]>>], cols: usize) -> Vec<Vec<[u8; 3]>> {
        if blocks.is_empty() || cols == 0 {
            return Vec::new();
        }
        let block_h = blocks[0].len();
        let block_w = if block_h > 0 { blocks[0][0].len() } else { 0 };
        let rows = (blocks.len() + cols - 1) / cols;
        let total_h = rows * block_h;
        let total_w = cols * block_w;

        let mut out: Vec<Vec<[u8; 3]>> = vec![vec![[0u8; 3]; total_w]; total_h];

        for (i, block) in blocks.iter().enumerate() {
            let block_row = i / cols;
            let block_col = i % cols;
            let y_off = block_row * block_h;
            let x_off = block_col * block_w;
            for (py, row) in block.iter().enumerate() {
                for (px, &color) in row.iter().enumerate() {
                    if y_off + py < total_h && x_off + px < total_w {
                        out[y_off + py][x_off + px] = color;
                    }
                }
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// QuiltGenerator
// ---------------------------------------------------------------------------

pub struct QuiltGenerator;

impl QuiltGenerator {
    /// Generate a quilt of `rows × cols` blocks.
    pub fn generate(
        block: QuiltBlock,
        cols: u32,
        rows: u32,
        palette: &ColorPalette,
        seed: u64,
    ) -> Vec<Vec<[u8; 3]>> {
        let total = (cols * rows) as usize;
        let mut blocks = Vec::with_capacity(total);
        let mut rng = Lcg::new(seed);

        for i in 0..total {
            let block_seed = seed
                .wrapping_add(i as u64)
                .wrapping_mul((rng.next_usize(usize::MAX) as u64).wrapping_add(1));
            let b = match block {
                QuiltBlock::NinePatches => QuiltPattern::generate_nine_patch(palette, block_seed),
                QuiltBlock::PinwheelBlock => QuiltPattern::generate_pinwheel(palette),
                QuiltBlock::StarBlock => Self::generate_star_block(palette, block_seed),
                QuiltBlock::BasketWeave => Self::generate_basket_weave(palette, block_seed),
                QuiltBlock::LogCabin => QuiltPattern::generate_log_cabin(palette, 3),
            };
            blocks.push(b);
        }

        QuiltPattern::assemble_quilt(&blocks, cols as usize)
    }

    /// Simple star block: diagonal cross on solid background.
    fn generate_star_block(palette: &ColorPalette, seed: u64) -> Vec<Vec<[u8; 3]>> {
        let size = 96usize;
        let bg = palette.get(0);
        let fg = palette.get(1);
        let accent = palette.get(seed as usize % palette.colors.len().max(1));
        let mut grid: Vec<Vec<[u8; 3]>> = vec![vec![bg; size]; size];
        let half = size / 2;
        let arm = size / 6;

        for py in 0..size {
            for px in 0..size {
                let dx = (px as i32 - half as i32).abs() as usize;
                let dy = (py as i32 - half as i32).abs() as usize;
                // Horizontal/vertical arms
                if dy < arm || dx < arm {
                    grid[py][px] = fg;
                }
                // Diagonal arms
                if dx == dy || (dx == dy + 1) || (dy == dx + 1) {
                    if dx < half {
                        grid[py][px] = accent;
                    }
                }
            }
        }
        grid
    }

    /// Basket weave: alternating horizontal and vertical stripes in 2×2 blocks.
    fn generate_basket_weave(palette: &ColorPalette, _seed: u64) -> Vec<Vec<[u8; 3]>> {
        let size = 80usize;
        let stripe = 10usize; // width of each stripe
        let c0 = palette.get(0);
        let c1 = palette.get(1);
        let mut grid: Vec<Vec<[u8; 3]>> = vec![vec![[0u8; 3]; size]; size];

        for py in 0..size {
            for px in 0..size {
                // Determine 2×2 basket cell
                let cell_row = py / (stripe * 2);
                let cell_col = px / (stripe * 2);
                // Alternate orientation per cell
                let horizontal = (cell_row + cell_col) % 2 == 0;
                let color = if horizontal {
                    // Horizontal stripes: alternate by row within cell
                    if (py / stripe) % 2 == 0 {
                        c0
                    } else {
                        c1
                    }
                } else {
                    // Vertical stripes: alternate by col within cell
                    if (px / stripe) % 2 == 0 {
                        c0
                    } else {
                        c1
                    }
                };
                grid[py][px] = color;
            }
        }
        grid
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn default_palette() -> ColorPalette {
        ColorPalette::default_palette()
    }

    #[test]
    fn test_block_sizes() {
        assert_eq!(QuiltBlock::NinePatches.block_size(), 90);
        assert_eq!(QuiltBlock::PinwheelBlock.block_size(), 80);
        assert_eq!(QuiltBlock::StarBlock.block_size(), 96);
        assert_eq!(QuiltBlock::BasketWeave.block_size(), 80);
        assert_eq!(QuiltBlock::LogCabin.block_size(), 100);
    }

    #[test]
    fn test_palette_get_wrapping() {
        let p = ColorPalette::new(vec![[1, 2, 3], [4, 5, 6]]);
        assert_eq!(p.get(0), [1, 2, 3]);
        assert_eq!(p.get(1), [4, 5, 6]);
        assert_eq!(p.get(2), [1, 2, 3]); // wraps
    }

    #[test]
    fn test_palette_empty() {
        let p = ColorPalette::new(vec![]);
        assert_eq!(p.get(0), [128, 128, 128]);
    }

    #[test]
    fn test_nine_patch_dimensions() {
        let grid = QuiltPattern::generate_nine_patch(&default_palette(), 42);
        assert_eq!(grid.len(), 90);
        assert!(grid.iter().all(|r| r.len() == 90));
    }

    #[test]
    fn test_nine_patch_has_multiple_colors() {
        let grid = QuiltPattern::generate_nine_patch(&default_palette(), 1);
        let first = grid[0][0];
        let has_other = grid.iter().flatten().any(|&c| c != first);
        assert!(has_other, "Nine patch should have more than one color");
    }

    #[test]
    fn test_pinwheel_dimensions() {
        let grid = QuiltPattern::generate_pinwheel(&default_palette());
        assert_eq!(grid.len(), 80);
        assert!(grid.iter().all(|r| r.len() == 80));
    }

    #[test]
    fn test_pinwheel_two_colors() {
        let p = ColorPalette::new(vec![[255, 0, 0], [0, 0, 255]]);
        let grid = QuiltPattern::generate_pinwheel(&p);
        let has_red = grid.iter().flatten().any(|&c| c == [255u8, 0, 0]);
        let has_blue = grid.iter().flatten().any(|&c| c == [0u8, 0, 255]);
        assert!(has_red && has_blue, "Pinwheel should use both colors");
    }

    #[test]
    fn test_log_cabin_nonempty() {
        let grid = QuiltPattern::generate_log_cabin(&default_palette(), 2);
        assert!(!grid.is_empty());
        assert!(!grid[0].is_empty());
    }

    #[test]
    fn test_assemble_quilt_dimensions() {
        let block_size = 10usize;
        let block: Vec<Vec<[u8; 3]>> = vec![vec![[1u8, 2, 3]; block_size]; block_size];
        let blocks = vec![block; 6];
        let assembled = QuiltPattern::assemble_quilt(&blocks, 3);
        assert_eq!(assembled.len(), 20); // 2 rows × 10
        assert_eq!(assembled[0].len(), 30); // 3 cols × 10
    }

    #[test]
    fn test_assemble_quilt_empty() {
        let result = QuiltPattern::assemble_quilt(&[], 3);
        assert!(result.is_empty());
    }

    #[test]
    fn test_quilt_generator_nine_patches() {
        let p = default_palette();
        let grid = QuiltGenerator::generate(QuiltBlock::NinePatches, 2, 2, &p, 99);
        // 2×2 grid of 90×90 blocks => 180×180
        assert_eq!(grid.len(), 180);
        assert_eq!(grid[0].len(), 180);
    }

    #[test]
    fn test_quilt_generator_log_cabin() {
        let p = default_palette();
        let grid = QuiltGenerator::generate(QuiltBlock::LogCabin, 1, 1, &p, 0);
        assert!(!grid.is_empty());
    }

    #[test]
    fn test_quilt_generator_basket_weave() {
        let p = default_palette();
        let grid = QuiltGenerator::generate(QuiltBlock::BasketWeave, 2, 1, &p, 5);
        assert!(!grid.is_empty());
        assert_eq!(grid[0].len(), 160); // 2 × 80
    }
}
