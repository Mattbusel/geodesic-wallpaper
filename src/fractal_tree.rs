//! L-system fractal tree with seasonal variations.

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

#[derive(Debug, Clone)]
pub struct TreeConfig {
    pub iterations: usize,
    pub angle_deg: f64,
    pub branch_ratio: f64,
    pub trunk_length: f64,
    pub season: Season,
    pub seed: u64,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            iterations: 6,
            angle_deg: 25.0,
            branch_ratio: 0.7,
            trunk_length: 100.0,
            season: Season::Summer,
            seed: 42,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TreeSegment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub depth: usize,
    pub color: [u8; 3],
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

/// Determine branch color based on depth, max_depth, and season.
pub fn branch_color(depth: usize, max_depth: usize, season: &Season) -> [u8; 3] {
    let t = if max_depth == 0 {
        1.0
    } else {
        depth as f64 / max_depth as f64
    };

    match season {
        Season::Spring => {
            // Brown trunk → green tips
            let r = (139.0 * (1.0 - t) + 34.0 * t) as u8;
            let g = (69.0 * (1.0 - t) + 180.0 * t) as u8;
            let b = (19.0 * (1.0 - t) + 34.0 * t) as u8;
            [r, g, b]
        }
        Season::Summer => {
            // Brown trunk → dark green
            let r = (101.0 * (1.0 - t) + 0.0 * t) as u8;
            let g = (67.0 * (1.0 - t) + 128.0 * t) as u8;
            let b = (33.0 * (1.0 - t) + 0.0 * t) as u8;
            [r, g, b]
        }
        Season::Autumn => {
            // Brown trunk → orange/red tips
            let r = (101.0 * (1.0 - t) + 220.0 * t) as u8;
            let g = (67.0 * (1.0 - t) + 80.0 * t) as u8;
            let b = (33.0 * (1.0 - t) + 10.0 * t) as u8;
            [r, g, b]
        }
        Season::Winter => {
            // Dark brown trunk → white/gray tips
            let r = (80.0 * (1.0 - t) + 220.0 * t) as u8;
            let g = (50.0 * (1.0 - t) + 220.0 * t) as u8;
            let b = (30.0 * (1.0 - t) + 230.0 * t) as u8;
            [r, g, b]
        }
    }
}

// ---------------------------------------------------------------------------
// Tree generation
// ---------------------------------------------------------------------------

struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
    }

    fn next_float(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.state >> 33) as f64 / (u32::MAX as f64 + 1.0)
    }

    /// Return jitter in degrees: ±5°
    fn jitter_deg(&mut self) -> f64 {
        (self.next_float() - 0.5) * 10.0
    }
}

fn generate_branch(
    x: f32,
    y: f32,
    angle_deg: f64,
    length: f64,
    depth: usize,
    max_depth: usize,
    config: &TreeConfig,
    rng: &mut LcgRng,
    segments: &mut Vec<TreeSegment>,
) {
    if depth > max_depth {
        return;
    }

    let angle_rad = angle_deg.to_radians();
    let x2 = x + (length * angle_rad.sin()) as f32;
    let y2 = y - (length * angle_rad.cos()) as f32; // y increases downward in image coords

    let color = branch_color(depth, max_depth, &config.season);
    segments.push(TreeSegment {
        x1: x,
        y1: y,
        x2,
        y2,
        depth,
        color,
    });

    if depth < max_depth {
        let next_length = length * config.branch_ratio;
        let jitter_l = rng.jitter_deg();
        let jitter_r = rng.jitter_deg();
        generate_branch(
            x2,
            y2,
            angle_deg + config.angle_deg + jitter_l,
            next_length,
            depth + 1,
            max_depth,
            config,
            rng,
            segments,
        );
        generate_branch(
            x2,
            y2,
            angle_deg - config.angle_deg + jitter_r,
            next_length,
            depth + 1,
            max_depth,
            config,
            rng,
            segments,
        );
    }
}

/// Generate all tree segments for the given config.
pub fn generate_tree(config: &TreeConfig) -> Vec<TreeSegment> {
    let mut segments = Vec::new();
    let mut rng = LcgRng::new(config.seed);
    // Start from bottom-center; angle 0 = straight up
    generate_branch(
        0.0,
        0.0,
        0.0,
        config.trunk_length,
        0,
        config.iterations,
        config,
        &mut rng,
        &mut segments,
    );
    segments
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render tree segments onto a pixel buffer using Bresenham line drawing.
pub fn render_tree(segments: &[TreeSegment], width: u32, height: u32, bg: [u8; 3]) -> Vec<u8> {
    let mut pixels = vec![0u8; (width * height * 3) as usize];
    // Fill background
    for i in 0..(width * height) as usize {
        pixels[i * 3] = bg[0];
        pixels[i * 3 + 1] = bg[1];
        pixels[i * 3 + 2] = bg[2];
    }

    // Offset so trunk starts at bottom-center
    let ox = (width / 2) as f32;
    let oy = (height - 10) as f32;

    for seg in segments {
        bresenham(
            (seg.x1 + ox) as i32,
            (seg.y1 + oy) as i32,
            (seg.x2 + ox) as i32,
            (seg.y2 + oy) as i32,
            seg.color,
            width,
            height,
            &mut pixels,
        );
    }
    pixels
}

fn bresenham(
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 3],
    width: u32,
    height: u32,
    pixels: &mut Vec<u8>,
) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let idx = ((y as u32 * width + x as u32) * 3) as usize;
            pixels[idx] = color[0];
            pixels[idx + 1] = color[1];
            pixels[idx + 2] = color[2];
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Return the endpoints of the deepest segments (leaf positions).
pub fn leaf_positions(segments: &[TreeSegment], max_depth: usize) -> Vec<(f32, f32)> {
    segments
        .iter()
        .filter(|s| s.depth == max_depth)
        .map(|s| (s.x2, s.y2))
        .collect()
}

/// Draw circles at leaf positions on the image.
pub fn add_leaves(
    image: &mut Vec<u8>,
    width: u32,
    leaves: &[(f32, f32)],
    radius: u32,
    color: [u8; 3],
) {
    let height = image.len() as u32 / (width * 3);
    let ox = (width / 2) as f32;
    let oy = (height - 10) as f32;

    for &(lx, ly) in leaves {
        let cx = (lx + ox) as i32;
        let cy = (ly + oy) as i32;
        let r = radius as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx * dx + dy * dy <= r * r {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        let idx = ((py as u32 * width + px as u32) * 3) as usize;
                        image[idx] = color[0];
                        image[idx + 1] = color[1];
                        image[idx + 2] = color[2];
                    }
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

    /// Number of segments for a binary tree of given depth:
    /// depth 0 → 1 segment (trunk)
    /// Each level doubles: total = 2^(depth+1) - 1
    fn expected_segments(iterations: usize) -> usize {
        (1usize << (iterations + 1)) - 1
    }

    #[test]
    fn test_generates_correct_segment_count() {
        for iters in [1, 2, 3, 4] {
            let config = TreeConfig {
                iterations: iters,
                angle_deg: 25.0,
                branch_ratio: 0.7,
                trunk_length: 100.0,
                season: Season::Summer,
                seed: 1,
            };
            let segments = generate_tree(&config);
            let expected = expected_segments(iters);
            assert_eq!(
                segments.len(),
                expected,
                "iterations={}: got {} segments, expected {}",
                iters,
                segments.len(),
                expected
            );
        }
    }

    #[test]
    fn test_render_returns_correct_size() {
        let config = TreeConfig {
            iterations: 3,
            ..TreeConfig::default()
        };
        let segments = generate_tree(&config);
        let pixels = render_tree(&segments, 200, 200, [255, 255, 255]);
        assert_eq!(pixels.len(), 200 * 200 * 3);
    }

    #[test]
    fn test_seasons_produce_different_colors() {
        let seasons = [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ];
        let colors: Vec<[u8; 3]> = seasons.iter().map(|s| branch_color(5, 5, s)).collect();
        // All four tip colors should differ
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(
                    colors[i], colors[j],
                    "Seasons {:?} and {:?} produce the same tip color",
                    seasons[i], seasons[j]
                );
            }
        }
    }

    #[test]
    fn test_leaf_positions_are_segment_endpoints() {
        let config = TreeConfig {
            iterations: 2,
            ..TreeConfig::default()
        };
        let segments = generate_tree(&config);
        let leaves = leaf_positions(&segments, config.iterations);
        // Each leaf position should match the x2,y2 of a depth=iterations segment
        for (lx, ly) in &leaves {
            let found = segments.iter().any(|s| {
                s.depth == config.iterations && (s.x2 - lx).abs() < 1e-4 && (s.y2 - ly).abs() < 1e-4
            });
            assert!(
                found,
                "Leaf position ({}, {}) not found in segments",
                lx, ly
            );
        }
    }
}
