//! 2D Cellular Automata: Game of Life, Rule 110, and Langton's Ant.

/// The state of a single cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellState {
    Dead,
    Alive,
    Custom(u8),
}

/// A 2D grid of cells with optional wrapping.
#[derive(Debug, Clone)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<u8>>,
    pub wrap: bool,
}

impl Grid {
    /// Creates a new empty (all-dead) grid.
    pub fn new(width: usize, height: usize, wrap: bool) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![0u8; width]; height],
            wrap,
        }
    }

    /// Creates a grid with ~30% alive cells using an LCG seeded PRNG.
    pub fn random(width: usize, height: usize, seed: u64) -> Self {
        let mut grid = Self::new(width, height, true);
        let mut state = seed.wrapping_add(1);
        for y in 0..height {
            for x in 0..width {
                // LCG: multiplier and increment from Numerical Recipes
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                let val = (state >> 33) & 0xFF;
                grid.cells[y][x] = if val < 77 { 1 } else { 0 }; // ~30% alive
            }
        }
        grid
    }

    /// Returns the wrapped coordinate.
    fn wrap_coord(&self, val: isize, max: usize) -> Option<usize> {
        if self.wrap {
            Some(val.rem_euclid(max as isize) as usize)
        } else if val >= 0 && val < max as isize {
            Some(val as usize)
        } else {
            None
        }
    }

    /// Counts alive neighbors in the Moore (8-cell) neighborhood.
    pub fn neighbor_count(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        for dy in -1_isize..=1 {
            for dx in -1_isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if let (Some(nx), Some(ny)) = (
                    self.wrap_coord(x as isize + dx, self.width),
                    self.wrap_coord(y as isize + dy, self.height),
                ) {
                    if self.cells[ny][nx] != 0 {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Advances the grid one step using Conway's Game of Life rules.
    pub fn game_of_life_step(&self) -> Self {
        let mut next = Self::new(self.width, self.height, self.wrap);
        for y in 0..self.height {
            for x in 0..self.width {
                let alive = self.cells[y][x] != 0;
                let neighbors = self.neighbor_count(x, y);
                next.cells[y][x] = match (alive, neighbors) {
                    (true, 2) | (true, 3) => 1,  // survives
                    (false, 3) => 1,               // born
                    _ => 0,                        // dies
                };
            }
        }
        next
    }

    /// Applies Rule 110 to each row independently (1D CA).
    pub fn rule_110_step(&self) -> Self {
        let mut next = Self::new(self.width, self.height, self.wrap);
        for y in 0..self.height {
            for x in 0..self.width {
                let left = if self.wrap {
                    self.cells[y][(x + self.width - 1) % self.width]
                } else if x > 0 {
                    self.cells[y][x - 1]
                } else {
                    0
                };
                let center = self.cells[y][x];
                let right = if self.wrap {
                    self.cells[y][(x + 1) % self.width]
                } else if x + 1 < self.width {
                    self.cells[y][x + 1]
                } else {
                    0
                };
                // Rule 110 lookup table
                let pattern = ((left & 1) << 2) | ((center & 1) << 1) | (right & 1);
                // Rule 110: 01101110 in binary, LSB = pattern 0
                const RULE_110: u8 = 0b01101110;
                next.cells[y][x] = (RULE_110 >> pattern) & 1;
            }
        }
        next
    }
}

/// Langton's Ant simulation.
pub struct LangtonAnt {
    pub x: i32,
    pub y: i32,
    pub direction: u8, // 0=N, 1=E, 2=S, 3=W
    pub grid: Grid,
}

impl LangtonAnt {
    /// Creates a new Langton's Ant starting at the center of the grid.
    pub fn new(grid: Grid) -> Self {
        let x = grid.width as i32 / 2;
        let y = grid.height as i32 / 2;
        Self { x, y, direction: 0, grid }
    }

    /// Executes one step of Langton's Ant.
    pub fn step(&mut self) {
        let gx = self.x.rem_euclid(self.grid.width as i32) as usize;
        let gy = self.y.rem_euclid(self.grid.height as i32) as usize;
        let cell = self.grid.cells[gy][gx];

        if cell == 0 {
            // White cell: turn right (clockwise), flip to black, move forward
            self.direction = (self.direction + 1) % 4;
            self.grid.cells[gy][gx] = 1;
        } else {
            // Black cell: turn left (counter-clockwise), flip to white, move forward
            self.direction = (self.direction + 3) % 4;
            self.grid.cells[gy][gx] = 0;
        }

        // Move forward
        match self.direction {
            0 => self.y -= 1, // North
            1 => self.x += 1, // East
            2 => self.y += 1, // South
            3 => self.x -= 1, // West
            _ => {}
        }
    }

    /// Runs the ant for a given number of steps.
    pub fn run(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
    }
}

/// Converts a grid to an RGB image buffer.
pub fn to_rgb(grid: &Grid, alive_color: [u8; 3], dead_color: [u8; 3]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(grid.width * grid.height * 3);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let color = if grid.cells[y][x] != 0 { alive_color } else { dead_color };
            buf.push(color[0]);
            buf.push(color[1]);
            buf.push(color[2]);
        }
    }
    buf
}

/// Converts a grid to a grayscale image buffer (0 or 255 per cell).
pub fn to_grayscale(grid: &Grid) -> Vec<u8> {
    let mut buf = Vec::with_capacity(grid.width * grid.height);
    for y in 0..grid.height {
        for x in 0..grid.width {
            buf.push(if grid.cells[y][x] != 0 { 255 } else { 0 });
        }
    }
    buf
}

/// Automaton rule type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CARule {
    GameOfLife,
    Rule110,
    LangtonAnt,
}

/// Animates a cellular automaton, producing grayscale image frames.
pub struct CAAnimator {
    pub grid: Grid,
    pub rule: CARule,
    pub frame_count: usize,
    langton_ant: Option<LangtonAnt>,
}

impl CAAnimator {
    pub fn new(grid: Grid, rule: CARule) -> Self {
        let langton_ant = if rule == CARule::LangtonAnt {
            Some(LangtonAnt::new(grid.clone()))
        } else {
            None
        };
        Self {
            grid,
            rule,
            frame_count: 0,
            langton_ant,
        }
    }

    /// Steps the automaton and returns a grayscale image.
    pub fn next_frame(&mut self) -> Vec<u8> {
        match self.rule {
            CARule::GameOfLife => {
                self.grid = self.grid.game_of_life_step();
            }
            CARule::Rule110 => {
                self.grid = self.grid.rule_110_step();
            }
            CARule::LangtonAnt => {
                if let Some(ant) = &mut self.langton_ant {
                    ant.step();
                    self.grid = ant.grid.clone();
                }
            }
        }
        self.frame_count += 1;
        to_grayscale(&self.grid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a glider pattern at offset (col, row).
    fn make_glider(width: usize, height: usize, col: usize, row: usize) -> Grid {
        let mut grid = Grid::new(width, height, true);
        // Standard glider:
        //  .X.
        //  ..X
        //  XXX
        let pattern = [(0, 1), (1, 2), (2, 0), (2, 1), (2, 2)];
        for (dy, dx) in &pattern {
            grid.cells[row + dy][col + dx] = 1;
        }
        grid
    }

    #[test]
    fn gol_glider_survives_one_step() {
        let grid = make_glider(10, 10, 1, 1);
        let next = grid.game_of_life_step();
        // The glider should still have 5 alive cells after 1 step
        let alive: usize = next.cells.iter().flat_map(|r| r.iter()).filter(|&&c| c != 0).count();
        assert_eq!(alive, 5, "Glider should still have 5 alive cells after 1 step");
    }

    #[test]
    fn rule_110_known_output() {
        // Single cell in center, Rule 110
        let mut grid = Grid::new(8, 1, true);
        grid.cells[0][4] = 1; // single 1 in the middle
        let next = grid.rule_110_step();
        // Rule 110 with pattern 010 (left=0, center=1, right=0) = pattern 2 -> bit 2 of 110 = 1
        // Rule 110 with pattern 000 = 0
        // The center cell (pattern 010) -> RULE_110 >> 2 & 1 = (0b01101110 >> 2) & 1 = 0b011011 & 1 = 1
        assert_eq!(next.cells[0][4], 1, "Center cell stays alive under Rule 110 with 010 pattern");
    }

    #[test]
    fn langton_ant_changes_cell_on_step() {
        let grid = Grid::new(20, 20, true);
        let mut ant = LangtonAnt::new(grid);
        let cx = ant.x as usize;
        let cy = ant.y as usize;
        let before = ant.grid.cells[cy][cx];
        ant.step();
        // After step, the cell the ant was on should be flipped
        let after = ant.grid.cells[cy][cx];
        assert_ne!(before, after, "Langton's Ant should flip the cell it stands on");
    }

    #[test]
    fn to_rgb_returns_correct_size() {
        let grid = Grid::new(10, 8, false);
        let buf = to_rgb(&grid, [255, 0, 0], [0, 0, 0]);
        assert_eq!(buf.len(), 10 * 8 * 3);
    }

    #[test]
    fn random_grid_has_some_alive_cells() {
        let grid = Grid::random(20, 20, 42);
        let alive: usize = grid.cells.iter().flat_map(|r| r.iter()).filter(|&&c| c != 0).count();
        assert!(alive > 0, "Random grid should have some alive cells");
        assert!(alive < 400, "Random grid should not be entirely alive");
    }
}
