//! Image dithering algorithms for wallpaper stylization.
//!
//! Provides Bayer ordered dithering, Floyd-Steinberg error diffusion, Atkinson
//! dithering, and nearest-colour palette reduction.

// ── Palette ───────────────────────────────────────────────────────────────────

/// A fixed colour palette for quantization.
#[derive(Debug, Clone)]
pub struct Palette {
    pub colors: Vec<(u8, u8, u8)>,
}

impl Palette {
    /// Find the palette colour nearest to `(r, g, b)` by Euclidean distance.
    pub fn find_nearest(&self, r: u8, g: u8, b: u8) -> (u8, u8, u8) {
        self.colors
            .iter()
            .cloned()
            .min_by_key(|&(cr, cg, cb)| {
                let dr = r as i32 - cr as i32;
                let dg = g as i32 - cg as i32;
                let db = b as i32 - cb as i32;
                dr * dr + dg * dg + db * db
            })
            .unwrap_or((0, 0, 0))
    }

    /// Monochrome: black and white.
    pub fn monochrome() -> Self {
        Palette { colors: vec![(0, 0, 0), (255, 255, 255)] }
    }

    /// Commodore 64 16-colour palette.
    pub fn c64() -> Self {
        Palette {
            colors: vec![
                (0,   0,   0  ), // Black
                (255, 255, 255), // White
                (136, 0,   0  ), // Red
                (170, 255, 238), // Cyan
                (204, 68,  204), // Purple
                (0,   204, 85 ), // Green
                (0,   0,   170), // Blue
                (238, 238, 119), // Yellow
                (221, 136, 85 ), // Orange
                (102, 68,  0  ), // Brown
                (255, 119, 119), // Light Red
                (51,  51,  51 ), // Dark Grey
                (119, 119, 119), // Grey
                (170, 255, 102), // Light Green
                (0,   136, 255), // Light Blue
                (187, 187, 187), // Light Grey
            ],
        }
    }

    /// Game Boy 4-shade green palette.
    pub fn gameboy() -> Self {
        Palette {
            colors: vec![
                (15,  56,  15 ), // Darkest green
                (48,  98,  48 ), // Dark green
                (139, 172, 15 ), // Light green
                (155, 188, 15 ), // Lightest green
            ],
        }
    }

    /// EGA 16-colour palette.
    pub fn ega() -> Self {
        Palette {
            colors: vec![
                (0,   0,   0  ), // Black
                (0,   0,   170), // Blue
                (0,   170, 0  ), // Green
                (0,   170, 170), // Cyan
                (170, 0,   0  ), // Red
                (170, 0,   170), // Magenta
                (170, 85,  0  ), // Brown
                (170, 170, 170), // Light Grey
                (85,  85,  85 ), // Dark Grey
                (85,  85,  255), // Bright Blue
                (85,  255, 85 ), // Bright Green
                (85,  255, 255), // Bright Cyan
                (255, 85,  85 ), // Bright Red
                (255, 85,  255), // Bright Magenta
                (255, 255, 85 ), // Bright Yellow
                (255, 255, 255), // White
            ],
        }
    }
}

// ── Bayer matrices ────────────────────────────────────────────────────────────

/// Return the normalised Bayer threshold matrix for size 2, 4, or 8.
fn bayer_matrix(n: usize) -> Vec<Vec<f64>> {
    match n {
        2 => vec![
            vec![0.0, 2.0],
            vec![3.0, 1.0],
        ],
        4 => vec![
            vec![ 0.0,  8.0,  2.0, 10.0],
            vec![12.0,  4.0, 14.0,  6.0],
            vec![ 3.0, 11.0,  1.0,  9.0],
            vec![15.0,  7.0, 13.0,  5.0],
        ],
        // 8×8 Bayer matrix.
        _ => {
            let b4 = bayer_matrix(4);
            let mut m = vec![vec![0.0f64; 8]; 8];
            for oy in 0..2usize {
                for ox in 0..2usize {
                    for iy in 0..4usize {
                        for ix in 0..4usize {
                            let base = [[0.0, 32.0], [48.0, 16.0]];
                            m[oy * 4 + iy][ox * 4 + ix] = 4.0 * b4[iy][ix] + base[oy][ox];
                        }
                    }
                }
            }
            m
        }
    }
}

// ── ordered_dither ────────────────────────────────────────────────────────────

/// Bayer matrix ordered dithering.
///
/// `matrix_size` controls the Bayer tile size (2, 4, or 8; defaults to 4).
pub fn ordered_dither(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
    matrix_size: usize,
) -> Vec<u8> {
    let n = match matrix_size {
        2 => 2,
        4 => 4,
        _ => 8,
    };
    let matrix = bayer_matrix(n);
    let n2 = (n * n) as f64;

    let mut out = vec![0u8; width * height * 3];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            let r = pixels[idx] as f64;
            let g = pixels[idx + 1] as f64;
            let b = pixels[idx + 2] as f64;

            let threshold = matrix[y % n][x % n] / n2 * 255.0;

            let (nr, ng, nb) = palette.find_nearest(
                (r + threshold).clamp(0.0, 255.0) as u8,
                (g + threshold).clamp(0.0, 255.0) as u8,
                (b + threshold).clamp(0.0, 255.0) as u8,
            );
            out[idx] = nr;
            out[idx + 1] = ng;
            out[idx + 2] = nb;
        }
    }
    out
}

// ── floyd_steinberg_dither ────────────────────────────────────────────────────

/// Floyd-Steinberg error-diffusion dithering.
pub fn floyd_steinberg_dither(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
) -> Vec<u8> {
    // Work in i32 to handle signed errors.
    let mut buf: Vec<[i32; 3]> = pixels
        .chunks_exact(3)
        .map(|c| [c[0] as i32, c[1] as i32, c[2] as i32])
        .collect();

    let mut out = vec![0u8; width * height * 3];

    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let r = buf[i][0].clamp(0, 255) as u8;
            let g = buf[i][1].clamp(0, 255) as u8;
            let b = buf[i][2].clamp(0, 255) as u8;

            let (qr, qg, qb) = palette.find_nearest(r, g, b);
            let idx = i * 3;
            out[idx] = qr;
            out[idx + 1] = qg;
            out[idx + 2] = qb;

            let er = r as i32 - qr as i32;
            let eg = g as i32 - qg as i32;
            let eb = b as i32 - qb as i32;

            // Distribute error to neighbors.
            let distribute = |buf: &mut Vec<[i32; 3]>, nx: usize, ny: usize, factor: i32| {
                let ni = ny * width + nx;
                buf[ni][0] += er * factor / 16;
                buf[ni][1] += eg * factor / 16;
                buf[ni][2] += eb * factor / 16;
            };

            if x + 1 < width {
                distribute(&mut buf, x + 1, y, 7);
            }
            if y + 1 < height {
                if x > 0 {
                    distribute(&mut buf, x - 1, y + 1, 3);
                }
                distribute(&mut buf, x, y + 1, 5);
                if x + 1 < width {
                    distribute(&mut buf, x + 1, y + 1, 1);
                }
            }
        }
    }
    out
}

// ── atkinson_dither ───────────────────────────────────────────────────────────

/// Atkinson dithering: distribute only 6/8 of error to 6 neighbours.
pub fn atkinson_dither(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
) -> Vec<u8> {
    let mut buf: Vec<[i32; 3]> = pixels
        .chunks_exact(3)
        .map(|c| [c[0] as i32, c[1] as i32, c[2] as i32])
        .collect();

    let mut out = vec![0u8; width * height * 3];

    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let r = buf[i][0].clamp(0, 255) as u8;
            let g = buf[i][1].clamp(0, 255) as u8;
            let b = buf[i][2].clamp(0, 255) as u8;

            let (qr, qg, qb) = palette.find_nearest(r, g, b);
            let idx = i * 3;
            out[idx] = qr;
            out[idx + 1] = qg;
            out[idx + 2] = qb;

            let er = r as i32 - qr as i32;
            let eg = g as i32 - qg as i32;
            let eb = b as i32 - qb as i32;

            // 6 neighbours, each gets 1/8 of the error.
            let neighbors: &[(i32, i32)] = &[
                (1, 0), (2, 0),
                (-1, 1), (0, 1), (1, 1),
                (0, 2),
            ];
            for &(dx, dy) in neighbors {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx >= 0 && nx < width as i32 && ny >= 0 && ny < height as i32 {
                    let ni = ny as usize * width + nx as usize;
                    buf[ni][0] += er / 8;
                    buf[ni][1] += eg / 8;
                    buf[ni][2] += eb / 8;
                }
            }
        }
    }
    out
}

// ── reduce_to_palette ─────────────────────────────────────────────────────────

/// Map each pixel to the nearest palette colour with no dithering.
pub fn reduce_to_palette(
    pixels: &[u8],
    width: usize,
    height: usize,
    palette: &Palette,
) -> Vec<u8> {
    let _ = (width, height); // dimensions unused but kept for API symmetry
    pixels
        .chunks_exact(3)
        .flat_map(|c| {
            let (r, g, b) = palette.find_nearest(c[0], c[1], c[2]);
            [r, g, b]
        })
        .collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn grey_image(w: usize, h: usize, v: u8) -> Vec<u8> {
        vec![v; w * h * 3]
    }

    #[test]
    fn bayer_matrix_2x2_correct() {
        let m = bayer_matrix(2);
        assert_eq!(m[0][0], 0.0);
        assert_eq!(m[0][1], 2.0);
        assert_eq!(m[1][0], 3.0);
        assert_eq!(m[1][1], 1.0);
    }

    #[test]
    fn floyd_steinberg_output_same_dimensions() {
        let w = 16;
        let h = 16;
        let pixels = grey_image(w, h, 128);
        let out = floyd_steinberg_dither(&pixels, w, h, &Palette::monochrome());
        assert_eq!(out.len(), w * h * 3);
    }

    #[test]
    fn monochrome_palette_only_produces_black_or_white() {
        let w = 8;
        let h = 8;
        let mut pixels = Vec::with_capacity(w * h * 3);
        for i in 0..(w * h) {
            let v = (i * 4 % 256) as u8;
            pixels.extend_from_slice(&[v, v, v]);
        }
        let out = ordered_dither(&pixels, w, h, &Palette::monochrome(), 4);
        for chunk in out.chunks_exact(3) {
            let (r, g, b) = (chunk[0], chunk[1], chunk[2]);
            assert!(
                (r == 0 && g == 0 && b == 0) || (r == 255 && g == 255 && b == 255),
                "unexpected colour ({r},{g},{b})"
            );
        }
    }

    #[test]
    fn ega_palette_has_16_colors() {
        let p = Palette::ega();
        assert_eq!(p.colors.len(), 16);
    }

    #[test]
    fn c64_palette_has_16_colors() {
        let p = Palette::c64();
        assert_eq!(p.colors.len(), 16);
    }

    #[test]
    fn atkinson_output_correct_size() {
        let w = 10;
        let h = 10;
        let pixels = grey_image(w, h, 100);
        let out = atkinson_dither(&pixels, w, h, &Palette::gameboy());
        assert_eq!(out.len(), w * h * 3);
    }

    #[test]
    fn reduce_to_palette_correct_size() {
        let w = 4;
        let h = 4;
        let pixels = grey_image(w, h, 200);
        let out = reduce_to_palette(&pixels, w, h, &Palette::monochrome());
        assert_eq!(out.len(), w * h * 3);
    }

    #[test]
    fn find_nearest_exact_match() {
        let p = Palette::monochrome();
        assert_eq!(p.find_nearest(0, 0, 0), (0, 0, 0));
        assert_eq!(p.find_nearest(255, 255, 255), (255, 255, 255));
    }
}
