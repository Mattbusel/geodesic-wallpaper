//! Photomosaic and pixelation effects.

/// How to sample the representative colour of a pixel block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Average,
    Median,
    Dominant,
}

/// Config for square pixelation.
#[derive(Debug, Clone)]
pub struct PixelateConfig {
    pub block_size: u32,
    pub blend_mode: BlendMode,
}

/// Config for mosaic (tile) filter.
#[derive(Debug, Clone)]
pub struct MosaicConfig {
    pub tile_size: u32,
    pub grid_cols: u32,
    pub grid_rows: u32,
}

/// Reduce k colours via simple k-means initialised with evenly-spaced picks.
pub struct ColorReduction {
    pub palette_size: u8,
}

// â”€â”€ helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn clamp_block(
    image: &Vec<Vec<[u8; 3]>>,
    row: usize,
    col: usize,
    bh: usize,
    bw: usize,
) -> Vec<[u8; 3]> {
    let h = image.len();
    let w = if h > 0 { image[0].len() } else { 0 };
    let mut pixels = Vec::new();
    for r in row..(row + bh).min(h) {
        for c in col..(col + bw).min(w) {
            pixels.push(image[r][c]);
        }
    }
    pixels
}

fn average_color(pixels: &[[u8; 3]]) -> [u8; 3] {
    if pixels.is_empty() { return [0, 0, 0]; }
    let (r, g, b) = pixels.iter().fold((0u64, 0u64, 0u64), |(r, g, b), p| {
        (r + p[0] as u64, g + p[1] as u64, b + p[2] as u64)
    });
    let n = pixels.len() as u64;
    [(r / n) as u8, (g / n) as u8, (b / n) as u8]
}

fn median_color(pixels: &[[u8; 3]]) -> [u8; 3] {
    if pixels.is_empty() { return [0, 0, 0]; }
    let mut rs: Vec<u8> = pixels.iter().map(|p| p[0]).collect();
    let mut gs: Vec<u8> = pixels.iter().map(|p| p[1]).collect();
    let mut bs: Vec<u8> = pixels.iter().map(|p| p[2]).collect();
    rs.sort_unstable(); gs.sort_unstable(); bs.sort_unstable();
    let m = pixels.len() / 2;
    [rs[m], gs[m], bs[m]]
}

fn dominant_color(pixels: &[[u8; 3]]) -> [u8; 3] {
    if pixels.is_empty() { return [0, 0, 0]; }
    // Quantise to 4-bit per channel, then find most frequent bucket.
    let mut counts: std::collections::HashMap<(u8, u8, u8), usize> = std::collections::HashMap::new();
    for p in pixels {
        let key = (p[0] >> 4, p[1] >> 4, p[2] >> 4);
        *counts.entry(key).or_insert(0) += 1;
    }
    let best = counts.into_iter().max_by_key(|&(_, c)| c).map(|(k, _)| k).unwrap();
    [best.0 << 4, best.1 << 4, best.2 << 4]
}

fn representative(pixels: &[[u8; 3]], mode: BlendMode) -> [u8; 3] {
    match mode {
        BlendMode::Average  => average_color(pixels),
        BlendMode::Median   => median_color(pixels),
        BlendMode::Dominant => dominant_color(pixels),
    }
}

fn color_dist_sq(a: [u8; 3], b: [u8; 3]) -> u64 {
    let dr = a[0] as i64 - b[0] as i64;
    let dg = a[1] as i64 - b[1] as i64;
    let db = a[2] as i64 - b[2] as i64;
    (dr * dr + dg * dg + db * db) as u64
}

// â”€â”€ public API â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Pixelate `image` by downsampling blocks then replacing every pixel in the
/// block with the representative colour.
pub fn pixelate(image: &Vec<Vec<[u8; 3]>>, config: &PixelateConfig) -> Vec<Vec<[u8; 3]>> {
    let h = image.len();
    if h == 0 { return vec![]; }
    let w = image[0].len();
    let bs = config.block_size.max(1) as usize;
    let mut out = image.clone();
    let mut row = 0;
    while row < h {
        let mut col = 0;
        while col < w {
            let pixels = clamp_block(image, row, col, bs, bs);
            let rep = representative(&pixels, config.blend_mode);
            for r in row..(row + bs).min(h) {
                for c in col..(col + bs).min(w) {
                    out[r][c] = rep;
                }
            }
            col += bs;
        }
        row += bs;
    }
    out
}

/// Replace each tile with the dominant colour of its pixels.
pub fn mosaic_filter(image: &Vec<Vec<[u8; 3]>>, config: &MosaicConfig) -> Vec<Vec<[u8; 3]>> {
    let h = image.len();
    if h == 0 { return vec![]; }
    let w = image[0].len();
    let ts = config.tile_size.max(1) as usize;
    let mut out = image.clone();
    let mut row = 0;
    while row < h {
        let mut col = 0;
        while col < w {
            let pixels = clamp_block(image, row, col, ts, ts);
            let rep = dominant_color(&pixels);
            for r in row..(row + ts).min(h) {
                for c in col..(col + ts).min(w) {
                    out[r][c] = rep;
                }
            }
            col += ts;
        }
        row += ts;
    }
    out
}

/// Pixelate using circular cells.
pub struct CircularPixelate;

impl CircularPixelate {
    pub fn render(image: &Vec<Vec<[u8; 3]>>, cell_radius: u32) -> Vec<Vec<[u8; 3]>> {
        let h = image.len();
        if h == 0 { return vec![]; }
        let w = image[0].len();
        let r = cell_radius.max(1) as usize;
        let diameter = r * 2;
        let mut out = image.clone();

        let mut cy = r;
        while cy < h + r {
            let mut cx = r;
            while cx < w + r {
                // Collect pixels within circle of radius r centred on (cy, cx)
                let mut pixels = Vec::new();
                let row_start = cy.saturating_sub(r);
                let col_start = cx.saturating_sub(r);
                for row in row_start..(cy + r).min(h) {
                    for col in col_start..(cx + r).min(w) {
                        let dr = row as isize - cy as isize;
                        let dc = col as isize - cx as isize;
                        if dr * dr + dc * dc <= (r * r) as isize {
                            pixels.push(image[row][col]);
                        }
                    }
                }
                let rep = average_color(&pixels);
                // Paint that colour back into the circle
                for row in row_start..(cy + r).min(h) {
                    for col in col_start..(cx + r).min(w) {
                        let dr = row as isize - cy as isize;
                        let dc = col as isize - cx as isize;
                        if dr * dr + dc * dc <= (r * r) as isize {
                            out[row][col] = rep;
                        }
                    }
                }
                cx += diameter;
            }
            cy += diameter;
        }
        out
    }
}

impl ColorReduction {
    pub fn new(palette_size: u8) -> Self {
        Self { palette_size }
    }

    /// Map every pixel to its nearest colour in `palette`.
    pub fn reduce(&self, image: &Vec<Vec<[u8; 3]>>, palette: &[[u8; 3]]) -> Vec<Vec<[u8; 3]>> {
        if palette.is_empty() { return image.clone(); }
        image.iter().map(|row| {
            row.iter().map(|&px| {
                *palette.iter().min_by_key(|&&p| color_dist_sq(px, p)).unwrap()
            }).collect()
        }).collect()
    }

    /// Extract a palette of `n` representative colours via k-means (max 20 iters).
    pub fn extract_palette(&self, image: &Vec<Vec<[u8; 3]>>, n: u8) -> Vec<[u8; 3]> {
        let k = n.max(1) as usize;
        let h = image.len();
        if h == 0 { return vec![[0, 0, 0]; k]; }
        let w = image[0].len();
        let total = h * w;
        if total == 0 { return vec![[0, 0, 0]; k]; }

        // Init centres evenly spaced across pixels
        let mut centres: Vec<[u8; 3]> = (0..k)
            .map(|i| {
                let idx = (i * total / k).min(total - 1);
                image[idx / w][idx % w]
            })
            .collect();

        for _ in 0..20 {
            let mut sums = vec![[0u64; 3]; k];
            let mut counts = vec![0u64; k];

            for row in image {
                for &px in row {
                    let ci = centres.iter().enumerate()
                        .min_by_key(|&(_, &c)| color_dist_sq(px, c))
                        .map(|(i, _)| i)
                        .unwrap();
                    sums[ci][0] += px[0] as u64;
                    sums[ci][1] += px[1] as u64;
                    sums[ci][2] += px[2] as u64;
                    counts[ci] += 1;
                }
            }

            let new_centres: Vec<[u8; 3]> = sums.iter().zip(&counts).enumerate().map(|(i, (s, &c))| {
                if c == 0 { centres[i] } else { [(s[0]/c) as u8, (s[1]/c) as u8, (s[2]/c) as u8] }
            }).collect();

            if new_centres == centres { break; }
            centres = new_centres;
        }
        centres
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid_image(h: usize, w: usize, color: [u8; 3]) -> Vec<Vec<[u8; 3]>> {
        vec![vec![color; w]; h]
    }

    fn gradient_image(h: usize, w: usize) -> Vec<Vec<[u8; 3]>> {
        (0..h).map(|r| (0..w).map(|c| [(r * 255 / h) as u8, (c * 255 / w) as u8, 128]).collect()).collect()
    }

    #[test]
    fn test_pixelate_solid() {
        let img = solid_image(8, 8, [100, 150, 200]);
        let cfg = PixelateConfig { block_size: 4, blend_mode: BlendMode::Average };
        let out = pixelate(&img, &cfg);
        assert_eq!(out[0][0], [100, 150, 200]);
    }

    #[test]
    fn test_pixelate_preserves_dims() {
        let img = gradient_image(16, 16);
        let cfg = PixelateConfig { block_size: 4, blend_mode: BlendMode::Median };
        let out = pixelate(&img, &cfg);
        assert_eq!(out.len(), 16);
        assert_eq!(out[0].len(), 16);
    }

    #[test]
    fn test_pixelate_dominant() {
        let img = solid_image(4, 4, [200, 200, 200]);
        let cfg = PixelateConfig { block_size: 2, blend_mode: BlendMode::Dominant };
        let out = pixelate(&img, &cfg);
        assert_eq!(out.len(), 4);
    }

    #[test]
    fn test_mosaic_filter_preserves_dims() {
        let img = gradient_image(16, 16);
        let cfg = MosaicConfig { tile_size: 4, grid_cols: 4, grid_rows: 4 };
        let out = mosaic_filter(&img, &cfg);
        assert_eq!(out.len(), 16);
        assert_eq!(out[0].len(), 16);
    }

    #[test]
    fn test_mosaic_filter_solid() {
        let img = solid_image(8, 8, [50, 60, 70]);
        let cfg = MosaicConfig { tile_size: 4, grid_cols: 2, grid_rows: 2 };
        let out = mosaic_filter(&img, &cfg);
        // dominant of solid block should round to nearest 16
        assert_eq!(out[0][0], out[4][4]);
    }

    #[test]
    fn test_circular_pixelate_dims() {
        let img = gradient_image(20, 20);
        let out = CircularPixelate::render(&img, 3);
        assert_eq!(out.len(), 20);
        assert_eq!(out[0].len(), 20);
    }

    #[test]
    fn test_color_reduction_reduce() {
        let img = solid_image(4, 4, [255, 0, 0]);
        let palette = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255]];
        let cr = ColorReduction::new(3);
        let out = cr.reduce(&img, &palette);
        assert_eq!(out[0][0], [255, 0, 0]);
    }

    #[test]
    fn test_extract_palette_count() {
        let img = gradient_image(16, 16);
        let cr = ColorReduction::new(4);
        let palette = cr.extract_palette(&img, 4);
        assert_eq!(palette.len(), 4);
    }

    #[test]
    fn test_empty_image_pixelate() {
        let img: Vec<Vec<[u8; 3]>> = vec![];
        let cfg = PixelateConfig { block_size: 4, blend_mode: BlendMode::Average };
        let out = pixelate(&img, &cfg);
        assert!(out.is_empty());
    }
}