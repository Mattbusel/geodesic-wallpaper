//! Wave Function Collapse: constraint-based procedural generation

use std::collections::VecDeque;

/// Adjacency rules: which tiles can appear next to each other
pub struct TileRules {
    pub n_tiles: usize,
    /// allowed_neighbors[tile][direction] = set of valid neighbor tile ids
    /// Directions: 0=North, 1=East, 2=South, 3=West
    pub allowed_neighbors: Vec<Vec<Vec<usize>>>,
    pub tile_weights: Vec<f64>,
}

impl TileRules {
    pub fn new(n_tiles: usize) -> Self {
        TileRules {
            n_tiles,
            allowed_neighbors: vec![vec![vec![]; 4]; n_tiles],
            tile_weights: vec![1.0; n_tiles],
        }
    }

    pub fn allow(mut self, tile_a: usize, dir: usize, tile_b: usize) -> Self {
        let opposite = [2, 3, 0, 1][dir]; // N<->S, E<->W
        self.allowed_neighbors[tile_a][dir].push(tile_b);
        self.allowed_neighbors[tile_b][opposite].push(tile_a);
        self
    }

    pub fn set_weight(mut self, tile: usize, weight: f64) -> Self {
        self.tile_weights[tile] = weight;
        self
    }
}

/// Create simple checkerboard-compatible rules (3 tiles: empty, wall, floor)
pub fn default_rules() -> TileRules {
    TileRules::new(3)
        .allow(0, 0, 0).allow(0, 1, 0).allow(0, 2, 0).allow(0, 3, 0)
        .allow(1, 0, 1).allow(1, 1, 1).allow(1, 2, 1).allow(1, 3, 1)
        .allow(2, 0, 2).allow(2, 1, 2).allow(2, 2, 2).allow(2, 3, 2)
        .allow(0, 0, 2).allow(0, 1, 2).allow(0, 2, 2).allow(0, 3, 2)
        .allow(1, 1, 2).allow(1, 3, 2)
}

pub struct WfcGrid {
    pub width: usize,
    pub height: usize,
    cells: Vec<Vec<usize>>, // possible tiles at each cell
    collapsed: Vec<bool>,
    rules: TileRules,
}

impl WfcGrid {
    pub fn new(width: usize, height: usize, rules: TileRules) -> Self {
        let all_tiles: Vec<usize> = (0..rules.n_tiles).collect();
        WfcGrid {
            width,
            height,
            cells: vec![all_tiles; width * height],
            collapsed: vec![false; width * height],
            rules,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize { y * self.width + x }

    /// Shannon entropy for a cell
    fn entropy(&self, x: usize, y: usize) -> f64 {
        let possibilities = &self.cells[self.idx(x, y)];
        if possibilities.len() <= 1 { return 0.0; }
        let weights: Vec<f64> = possibilities.iter().map(|&t| self.rules.tile_weights[t]).collect();
        let total: f64 = weights.iter().sum();
        -weights.iter().map(|&w| {
            let p = w / total;
            if p > 0.0 { p * p.ln() } else { 0.0 }
        }).sum::<f64>()
    }

    /// Find cell with lowest non-zero entropy
    fn min_entropy_cell(&self, seed: u64) -> Option<(usize, usize)> {
        let mut best = f64::MAX;
        let mut best_cells = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                if !self.collapsed[self.idx(x, y)] {
                    let e = self.entropy(x, y);
                    if e > 0.0 {
                        if e < best - 1e-10 { best = e; best_cells.clear(); }
                        if (e - best).abs() < 1e-10 { best_cells.push((x, y)); }
                    }
                }
            }
        }
        if best_cells.is_empty() { return None; }
        Some(best_cells[seed as usize % best_cells.len()])
    }

    /// Collapse a cell to one tile (weighted random selection)
    fn collapse_cell(&mut self, x: usize, y: usize, seed: u64) {
        let idx = self.idx(x, y);
        let possibilities = &self.cells[idx];
        let weights: Vec<f64> = possibilities.iter().map(|&t| self.rules.tile_weights[t]).collect();
        let total: f64 = weights.iter().sum();
        let mut r = (seed as f64 / u64::MAX as f64) * total;
        let chosen = possibilities.iter().zip(weights.iter())
            .find(|(_, &w)| { r -= w; r <= 0.0 })
            .map(|(&t, _)| t)
            .unwrap_or(possibilities[0]);
        self.cells[idx] = vec![chosen];
        self.collapsed[idx] = true;
    }

    /// Propagate constraints via AC-3
    fn propagate(&mut self, start_x: usize, start_y: usize) -> bool {
        let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
        queue.push_back((start_x, start_y));

        while let Some((cx, cy)) = queue.pop_front() {
            let neighbors: Vec<(usize, usize, usize)> = [
                (cy.wrapping_sub(1), cx, 0), // North: (y-1, x, dir=0)
                (cy, cx + 1, 1),              // East
                (cy + 1, cx, 2),              // South
                (cy, cx.wrapping_sub(1), 3),  // West
            ].iter().filter_map(|&(ny, nx, dir)| {
                if nx < self.width && ny < self.height { Some((nx, ny, dir)) }
                else { None }
            }).collect();

            for (nx, ny, dir) in neighbors {
                let ni = self.idx(nx, ny);
                let ci = self.idx(cx, cy);
                let current_tiles = self.cells[ci].clone();

                let before_len = self.cells[ni].len();
                self.cells[ni].retain(|&neighbor_tile| {
                    current_tiles.iter().any(|&ct| self.rules.allowed_neighbors[ct][dir].contains(&neighbor_tile))
                });

                if self.cells[ni].is_empty() { return false; } // contradiction
                if self.cells[ni].len() < before_len { queue.push_back((nx, ny)); }
            }
        }
        true
    }

    /// Run WFC until fully collapsed or contradiction
    pub fn run(&mut self, seed: u64) -> bool {
        let mut state = seed;
        loop {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            match self.min_entropy_cell(state) {
                None => return true, // all collapsed
                Some((x, y)) => {
                    self.collapse_cell(x, y, state >> 11);
                    if !self.propagate(x, y) { return false; } // contradiction
                }
            }
        }
    }

    /// Get final tile map (returns 0 if cell has no possibilities)
    pub fn result(&self) -> Vec<usize> {
        self.cells.iter().map(|c| c.first().copied().unwrap_or(0)).collect()
    }

    /// Render as RGBA pixels with tile colors
    pub fn render(&self, tile_size: u32, colors: &[(u8, u8, u8)]) -> Vec<u8> {
        let pw = self.width as u32 * tile_size;
        let ph = self.height as u32 * tile_size;
        let tiles = self.result();
        let mut pixels = vec![0u8; (pw * ph * 4) as usize];
        for cy in 0..self.height {
            for cx in 0..self.width {
                let tile = tiles[self.idx(cx, cy)];
                let (r, g, b) = colors.get(tile % colors.len()).copied().unwrap_or((128, 128, 128));
                for ty in 0..tile_size {
                    for tx in 0..tile_size {
                        let px = cx as u32 * tile_size + tx;
                        let py = cy as u32 * tile_size + ty;
                        let pi = ((py * pw + px) * 4) as usize;
                        if pi + 3 < pixels.len() {
                            pixels[pi] = r; pixels[pi+1] = g; pixels[pi+2] = b; pixels[pi+3] = 255;
                        }
                    }
                }
            }
        }
        pixels
    }
}
