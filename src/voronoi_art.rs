//! Artistic Voronoi diagram generation with styled cells.

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Site {
    pub x: f64,
    pub y: f64,
    pub color: [u8; 3],
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct VoronoiCell {
    pub site: Site,
    pub pixel_count: u32,
}

#[derive(Debug, Clone)]
pub enum ColorScheme {
    Pastel,
    Vibrant,
    Monochrome,
    Sunset,
    Ocean,
}

/// Convert HSV color to RGB.
pub fn hsv_to_rgb(h: f64, s: f64, v: f64) -> [u8; 3] {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [
        ((r1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b1 + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Compute a color for a given scheme, index, and total count.
pub fn color_for_scheme(scheme: &ColorScheme, index: usize, total: usize) -> [u8; 3] {
    let total = total.max(1);
    let t = index as f64 / total as f64;

    match scheme {
        ColorScheme::Pastel => {
            let h = t * 360.0;
            hsv_to_rgb(h, 0.4, 0.9)
        }
        ColorScheme::Vibrant => {
            let h = t * 360.0;
            hsv_to_rgb(h, 0.9, 0.9)
        }
        ColorScheme::Monochrome => {
            let v = (t * 220.0 + 20.0).clamp(0.0, 255.0) as u8;
            [v, v, v]
        }
        ColorScheme::Sunset => {
            // red → orange → purple
            if t < 0.5 {
                // red → orange
                let f = t * 2.0;
                [255, (f * 165.0) as u8, 0]
            } else {
                // orange → purple
                let f = (t - 0.5) * 2.0;
                let r = (255.0 * (1.0 - f) + 128.0 * f) as u8;
                let g = (165.0 * (1.0 - f)) as u8;
                let b = (128.0 * f) as u8;
                [r, g, b]
            }
        }
        ColorScheme::Ocean => {
            // dark_blue → cyan
            let dark_blue = [0u8, 30u8, 100u8];
            let cyan = [0u8, 200u8, 230u8];
            [
                (dark_blue[0] as f64 * (1.0 - t) + cyan[0] as f64 * t) as u8,
                (dark_blue[1] as f64 * (1.0 - t) + cyan[1] as f64 * t) as u8,
                (dark_blue[2] as f64 * (1.0 - t) + cyan[2] as f64 * t) as u8,
            ]
        }
    }
}

// ---------------------------------------------------------------------------
// VoronoiArt
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct VoronoiArtConfig {
    pub num_sites: usize,
    pub scheme: ColorScheme,
    pub border_width: u32,
    pub border_color: [u8; 3],
    pub weighted: bool,
}

impl Default for VoronoiArtConfig {
    fn default() -> Self {
        Self {
            num_sites: 20,
            scheme: ColorScheme::Pastel,
            border_width: 2,
            border_color: [0, 0, 0],
            weighted: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VoronoiArt {
    pub config: VoronoiArtConfig,
    pub sites: Vec<Site>,
}

impl VoronoiArt {
    /// Generate a VoronoiArt with randomly placed sites using LCG.
    pub fn generate(config: VoronoiArtConfig, width: u32, height: u32, seed: u64) -> Self {
        let mut state = seed.wrapping_add(1);
        let mut lcg = move || -> f64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 33) as f64 / (u32::MAX as f64 + 1.0)
        };

        let num_sites = config.num_sites;
        let mut sites = Vec::with_capacity(num_sites);
        for i in 0..num_sites {
            let x = lcg() * width as f64;
            let y = lcg() * height as f64;
            let weight = 0.5 + lcg() * 0.5; // [0.5, 1.0)
            let color = color_for_scheme(&config.scheme, i, num_sites);
            sites.push(Site {
                x,
                y,
                color,
                weight,
            });
        }

        Self { config, sites }
    }

    /// Find the index of the nearest site to point (x, y).
    /// If weighted=true, minimize dist/sqrt(weight).
    pub fn nearest_site(x: f64, y: f64, sites: &[Site], weighted: bool) -> usize {
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        for (i, site) in sites.iter().enumerate() {
            let dx = x - site.x;
            let dy = y - site.y;
            let dist_sq = dx * dx + dy * dy;
            let effective = if weighted {
                let w = site.weight.max(1e-10).sqrt();
                dist_sq / (w * w)
            } else {
                dist_sq
            };
            if effective < best_dist {
                best_dist = effective;
                best_idx = i;
            }
        }
        best_idx
    }

    /// Render the Voronoi diagram as an RGB pixel buffer.
    pub fn render(&self, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 3) as usize];
        let border = self.config.border_width as f64;

        for py in 0..height {
            for px in 0..width {
                let x = px as f64;
                let y = py as f64;
                let nearest = Self::nearest_site(x, y, &self.sites, self.config.weighted);

                // Check if this pixel is within border_width of a different site's territory
                let is_border = if border > 0.0 {
                    let mut found_other = false;
                    'outer: for ny in -(border as i32)..=(border as i32) {
                        for nx in -(border as i32)..=(border as i32) {
                            if nx == 0 && ny == 0 {
                                continue;
                            }
                            let bx = x + nx as f64;
                            let by = y + ny as f64;
                            if bx < 0.0 || bx >= width as f64 || by < 0.0 || by >= height as f64 {
                                continue;
                            }
                            let n2 = Self::nearest_site(bx, by, &self.sites, self.config.weighted);
                            if n2 != nearest {
                                found_other = true;
                                break 'outer;
                            }
                        }
                    }
                    found_other
                } else {
                    false
                };

                let color = if is_border {
                    self.config.border_color
                } else {
                    self.sites[nearest].color
                };

                let idx = ((py * width + px) * 3) as usize;
                pixels[idx] = color[0];
                pixels[idx + 1] = color[1];
                pixels[idx + 2] = color[2];
            }
        }
        pixels
    }

    /// Perform Lloyd's relaxation: move each site to the centroid of its Voronoi region.
    pub fn lloyd_relax(&mut self, width: u32, height: u32, iterations: usize) {
        for _ in 0..iterations {
            let n = self.sites.len();
            let mut sums_x = vec![0.0f64; n];
            let mut sums_y = vec![0.0f64; n];
            let mut counts = vec![0u32; n];

            for py in 0..height {
                for px in 0..width {
                    let nearest =
                        Self::nearest_site(px as f64, py as f64, &self.sites, self.config.weighted);
                    sums_x[nearest] += px as f64;
                    sums_y[nearest] += py as f64;
                    counts[nearest] += 1;
                }
            }

            for i in 0..n {
                if counts[i] > 0 {
                    self.sites[i].x = sums_x[i] / counts[i] as f64;
                    self.sites[i].y = sums_y[i] / counts[i] as f64;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_correct_buffer_size() {
        let config = VoronoiArtConfig {
            num_sites: 5,
            scheme: ColorScheme::Pastel,
            border_width: 1,
            border_color: [0, 0, 0],
            weighted: false,
        };
        let art = VoronoiArt::generate(config, 64, 64, 42);
        let pixels = art.render(64, 64);
        assert_eq!(pixels.len(), 64 * 64 * 3);
    }

    #[test]
    fn test_nearest_site_correctness() {
        let sites = vec![
            Site {
                x: 0.0,
                y: 0.0,
                color: [255, 0, 0],
                weight: 1.0,
            },
            Site {
                x: 100.0,
                y: 0.0,
                color: [0, 255, 0],
                weight: 1.0,
            },
        ];
        assert_eq!(VoronoiArt::nearest_site(10.0, 0.0, &sites, false), 0);
        assert_eq!(VoronoiArt::nearest_site(90.0, 0.0, &sites, false), 1);
    }

    #[test]
    fn test_lloyd_relax_moves_sites() {
        let config = VoronoiArtConfig {
            num_sites: 4,
            scheme: ColorScheme::Vibrant,
            border_width: 0,
            border_color: [0, 0, 0],
            weighted: false,
        };
        let mut art = VoronoiArt::generate(config, 32, 32, 99);
        let before: Vec<(f64, f64)> = art.sites.iter().map(|s| (s.x, s.y)).collect();
        art.lloyd_relax(32, 32, 1);
        let after: Vec<(f64, f64)> = art.sites.iter().map(|s| (s.x, s.y)).collect();
        // At least some sites should move
        let any_moved = before
            .iter()
            .zip(after.iter())
            .any(|((bx, by), (ax, ay))| (bx - ax).abs() > 1e-6 || (by - ay).abs() > 1e-6);
        assert!(any_moved, "Lloyd relaxation should move at least one site");
    }

    #[test]
    fn test_hsv_to_rgb_red() {
        // H=0 → red
        let rgb = hsv_to_rgb(0.0, 1.0, 1.0);
        assert_eq!(rgb, [255, 0, 0]);
    }

    #[test]
    fn test_hsv_to_rgb_green() {
        // H=120 → green
        let rgb = hsv_to_rgb(120.0, 1.0, 1.0);
        assert_eq!(rgb, [0, 255, 0]);
    }

    #[test]
    fn test_hsv_to_rgb_blue() {
        // H=240 → blue
        let rgb = hsv_to_rgb(240.0, 1.0, 1.0);
        assert_eq!(rgb, [0, 0, 255]);
    }
}
