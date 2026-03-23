//! Voronoi diagram with Lloyd's relaxation and Poisson disk sampling.
//!
//! Provides nearest-site Voronoi rendering, border detection, iterative
//! Lloyd relaxation, and Bridson-style Poisson disk sampling.

// ── Point ─────────────────────────────────────────────────────────────────────

/// A 2D point in continuous space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Squared Euclidean distance to another point.
    #[inline]
    pub fn dist_sq(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }

    /// Euclidean distance to another point.
    #[inline]
    pub fn dist(&self, other: &Point) -> f64 {
        self.dist_sq(other).sqrt()
    }
}

// ── VoronoiCell ───────────────────────────────────────────────────────────────

/// A Voronoi site with an associated RGB color.
#[derive(Debug, Clone)]
pub struct VoronoiCell {
    /// The generating site.
    pub site: Point,
    /// RGB colour for pixels belonging to this cell.
    pub color: (u8, u8, u8),
}

// ── VoronoiDiagram ────────────────────────────────────────────────────────────

/// A Voronoi diagram over an integer pixel grid.
pub struct VoronoiDiagram {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<VoronoiCell>,
}

impl VoronoiDiagram {
    /// Create a new diagram.
    pub fn new(
        width: usize,
        height: usize,
        sites: Vec<Point>,
        colors: Vec<(u8, u8, u8)>,
    ) -> Self {
        assert_eq!(sites.len(), colors.len(), "sites and colors must have equal length");
        let cells = sites
            .into_iter()
            .zip(colors)
            .map(|(site, color)| VoronoiCell { site, color })
            .collect();
        Self { width, height, cells }
    }

    /// Render the diagram: for each pixel, assign the colour of the nearest site.
    ///
    /// Returns a flat RGB buffer of length `width * height * 3`.
    pub fn render(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.width * self.height * 3];
        for py in 0..self.height {
            for px in 0..self.width {
                let p = Point::new(px as f64 + 0.5, py as f64 + 0.5);
                let (cell_idx, _) = self.nearest_site(&p);
                let color = self.cells[cell_idx].color;
                let base = (py * self.width + px) * 3;
                out[base] = color.0;
                out[base + 1] = color.1;
                out[base + 2] = color.2;
            }
        }
        out
    }

    /// Render with borders: pixels where the nearest and second-nearest site
    /// distances differ by less than `threshold` receive `border_color`.
    pub fn with_borders(&self, border_color: (u8, u8, u8), threshold: f64) -> Vec<u8> {
        let mut out = vec![0u8; self.width * self.height * 3];
        for py in 0..self.height {
            for px in 0..self.width {
                let p = Point::new(px as f64 + 0.5, py as f64 + 0.5);
                let (cell_idx, d1, d2) = self.two_nearest_sites(&p);
                let base = (py * self.width + px) * 3;
                let color = if (d2 - d1) < threshold {
                    border_color
                } else {
                    self.cells[cell_idx].color
                };
                out[base] = color.0;
                out[base + 1] = color.1;
                out[base + 2] = color.2;
            }
        }
        out
    }

    /// Find the index of the nearest site and its distance.
    fn nearest_site(&self, p: &Point) -> (usize, f64) {
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        for (i, cell) in self.cells.iter().enumerate() {
            let d = p.dist(&cell.site);
            if d < best_dist {
                best_dist = d;
                best_idx = i;
            }
        }
        (best_idx, best_dist)
    }

    /// Find nearest site index, its distance, and the second-nearest distance.
    fn two_nearest_sites(&self, p: &Point) -> (usize, f64, f64) {
        let mut best_idx = 0;
        let mut best_dist = f64::MAX;
        let mut second_dist = f64::MAX;
        for (i, cell) in self.cells.iter().enumerate() {
            let d = p.dist(&cell.site);
            if d < best_dist {
                second_dist = best_dist;
                best_dist = d;
                best_idx = i;
            } else if d < second_dist {
                second_dist = d;
            }
        }
        (best_idx, best_dist, second_dist)
    }
}

// ── LloydRelaxation ───────────────────────────────────────────────────────────

/// Lloyd's relaxation: iteratively move sites to the centroid of their Voronoi cells.
pub struct LloydRelaxation;

impl LloydRelaxation {
    /// Relax `sites` by `iterations` rounds within the given bounding box.
    ///
    /// Uses a pixel-space approach at 1-pixel resolution for speed.
    pub fn relax(
        sites: &[Point],
        width: f64,
        height: f64,
        iterations: usize,
    ) -> Vec<Point> {
        let w = width as usize;
        let h = height as usize;
        let n = sites.len();
        let mut current: Vec<Point> = sites.to_vec();

        for _ in 0..iterations {
            let mut sum_x = vec![0.0_f64; n];
            let mut sum_y = vec![0.0_f64; n];
            let mut count = vec![0usize; n];

            // Assign each pixel to nearest site.
            for py in 0..h {
                for px in 0..w {
                    let p = Point::new(px as f64 + 0.5, py as f64 + 0.5);
                    let mut best_idx = 0;
                    let mut best_dist = f64::MAX;
                    for (i, site) in current.iter().enumerate() {
                        let d = p.dist_sq(site);
                        if d < best_dist {
                            best_dist = d;
                            best_idx = i;
                        }
                    }
                    sum_x[best_idx] += p.x;
                    sum_y[best_idx] += p.y;
                    count[best_idx] += 1;
                }
            }

            // Move each site to centroid; if count==0, keep position.
            for i in 0..n {
                if count[i] > 0 {
                    current[i] = Point::new(
                        sum_x[i] / count[i] as f64,
                        sum_y[i] / count[i] as f64,
                    );
                }
            }
        }

        current
    }
}

// ── Poisson disk sampling ─────────────────────────────────────────────────────

/// Generate a set of points with minimum pairwise distance `min_dist` using
/// Bridson's algorithm (active-list approach with LCG random number generation).
pub fn generate_poisson_disk(width: f64, height: f64, min_dist: f64, seed: u64) -> Vec<Point> {
    let k = 30usize; // Maximum attempts per active sample.
    let cell_size = min_dist / std::f64::consts::SQRT_2;
    let grid_w = (width / cell_size).ceil() as usize + 1;
    let grid_h = (height / cell_size).ceil() as usize + 1;

    // Grid stores index+1 into `samples` (0 = empty).
    let mut grid = vec![0usize; grid_w * grid_h];
    let mut samples: Vec<Point> = Vec::new();
    let mut active: Vec<usize> = Vec::new();

    let mut rng = seed.wrapping_add(1);

    let lcg_next = |rng: &mut u64| -> f64 {
        *rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (*rng >> 11) as f64 / (1u64 << 53) as f64
    };

    let grid_idx = |p: &Point| -> Option<usize> {
        let gx = (p.x / cell_size) as usize;
        let gy = (p.y / cell_size) as usize;
        if gx < grid_w && gy < grid_h {
            Some(gy * grid_w + gx)
        } else {
            None
        }
    };

    // Initial sample near centre.
    let first = Point::new(width * 0.5, height * 0.5);
    if let Some(gi) = grid_idx(&first) {
        grid[gi] = 1;
        samples.push(first);
        active.push(0);
    }

    'outer: while !active.is_empty() {
        // Pick a random active sample.
        let r = (lcg_next(&mut rng) * active.len() as f64) as usize;
        let r = r.min(active.len() - 1);
        let parent_idx = active[r];
        let parent = samples[parent_idx];

        let mut found = false;
        for _ in 0..k {
            // Sample annulus [min_dist, 2*min_dist].
            let angle = lcg_next(&mut rng) * std::f64::consts::TAU;
            let radius = min_dist + lcg_next(&mut rng) * min_dist;
            let candidate = Point::new(
                parent.x + radius * angle.cos(),
                parent.y + radius * angle.sin(),
            );

            if candidate.x < 0.0 || candidate.x >= width
                || candidate.y < 0.0 || candidate.y >= height
            {
                continue;
            }

            // Check grid neighbourhood for conflicts.
            let gcx = (candidate.x / cell_size) as isize;
            let gcy = (candidate.y / cell_size) as isize;
            let mut conflict = false;

            'check: for dy in -2isize..=2 {
                for dx in -2isize..=2 {
                    let nx = gcx + dx;
                    let ny = gcy + dy;
                    if nx < 0 || ny < 0 || nx >= grid_w as isize || ny >= grid_h as isize {
                        continue;
                    }
                    let ni = (ny as usize) * grid_w + nx as usize;
                    let sample_idx = grid[ni];
                    if sample_idx > 0 {
                        let neighbor = samples[sample_idx - 1];
                        if candidate.dist(&neighbor) < min_dist {
                            conflict = true;
                            break 'check;
                        }
                    }
                }
            }

            if !conflict {
                if let Some(gi) = grid_idx(&candidate) {
                    let new_idx = samples.len();
                    samples.push(candidate);
                    active.push(new_idx);
                    grid[gi] = new_idx + 1;
                    found = true;
                    break;
                }
            }

            // Guard: if we have enough samples, stop early.
            if samples.len() > 100_000 {
                break 'outer;
            }
        }

        if !found {
            active.swap_remove(r);
        }
    }

    samples
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sites() -> Vec<Point> {
        vec![
            Point::new(25.0, 25.0),
            Point::new(75.0, 25.0),
            Point::new(25.0, 75.0),
            Point::new(75.0, 75.0),
        ]
    }

    fn make_colors() -> Vec<(u8, u8, u8)> {
        vec![(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0)]
    }

    #[test]
    fn render_output_dimensions() {
        let diagram = VoronoiDiagram::new(100, 100, make_sites(), make_colors());
        let pixels = diagram.render();
        assert_eq!(pixels.len(), 100 * 100 * 3);
    }

    #[test]
    fn with_borders_output_dimensions() {
        let diagram = VoronoiDiagram::new(100, 100, make_sites(), make_colors());
        let pixels = diagram.with_borders((0, 0, 0), 3.0);
        assert_eq!(pixels.len(), 100 * 100 * 3);
    }

    #[test]
    fn nearest_site_distance_correctness() {
        let sites = vec![Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
        let colors = vec![(255, 0, 0), (0, 255, 0)];
        let diagram = VoronoiDiagram::new(20, 10, sites, colors);

        // Point (1, 0) is closer to site 0 at (0,0) than site 1 at (10,0).
        let p = Point::new(1.5, 0.5);
        let (idx, dist) = diagram.nearest_site(&p);
        assert_eq!(idx, 0);
        assert!(dist < 10.0);
    }

    #[test]
    fn render_pixel_has_site_color() {
        let sites = vec![Point::new(10.0, 10.0), Point::new(90.0, 90.0)];
        let colors = vec![(255, 0, 0), (0, 0, 255)];
        let diagram = VoronoiDiagram::new(100, 100, sites, colors);
        let pixels = diagram.render();

        // Pixel (10, 10) should be very close to site 0 (red).
        let base = (10 * 100 + 10) * 3;
        assert_eq!(pixels[base], 255); // red channel
        assert_eq!(pixels[base + 2], 0); // blue channel
    }

    #[test]
    fn lloyd_relaxation_sites_move() {
        let sites = vec![
            Point::new(1.0, 1.0),
            Point::new(99.0, 99.0),
        ];
        let relaxed = LloydRelaxation::relax(&sites, 100.0, 100.0, 3);
        assert_eq!(relaxed.len(), 2);
        // Sites should have moved toward centroids.
        assert!(
            relaxed[0].x != sites[0].x || relaxed[0].y != sites[0].y
                || relaxed[1].x != sites[1].x || relaxed[1].y != sites[1].y,
            "sites should move during Lloyd relaxation"
        );
    }

    #[test]
    fn lloyd_relaxation_count_preserved() {
        let sites = make_sites();
        let relaxed = LloydRelaxation::relax(&sites, 100.0, 100.0, 2);
        assert_eq!(relaxed.len(), sites.len());
    }

    #[test]
    fn poisson_disk_min_distance_respected() {
        let min_dist = 15.0;
        let points = generate_poisson_disk(100.0, 100.0, min_dist, 42);
        assert!(!points.is_empty());
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let d = points[i].dist(&points[j]);
                assert!(
                    d >= min_dist - 1e-9,
                    "points {} and {} are too close: {:.4} < {:.4}",
                    i, j, d, min_dist
                );
            }
        }
    }

    #[test]
    fn poisson_disk_points_in_bounds() {
        let (w, h) = (100.0, 80.0);
        let points = generate_poisson_disk(w, h, 10.0, 1337);
        for p in &points {
            assert!(p.x >= 0.0 && p.x < w, "x out of bounds: {}", p.x);
            assert!(p.y >= 0.0 && p.y < h, "y out of bounds: {}", p.y);
        }
    }

    #[test]
    fn point_dist_correct() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!((a.dist(&b) - 5.0).abs() < 1e-9);
    }
}
