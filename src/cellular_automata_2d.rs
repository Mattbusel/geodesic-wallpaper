//! 2D Conway's Game of Life and variants.
//!
//! Implements a generic B/S (birth/survival) rule system that covers Conway,
//! HighLife, Day & Night, Seeds, and Mazectric.  All randomness uses a simple
//! LCG so there are zero external dependencies beyond `std`.

// ── CellState ─────────────────────────────────────────────────────────────────

/// The state of a single cell on the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellState {
    Dead,
    Alive,
}

impl CellState {
    fn is_alive(self) -> bool {
        self == CellState::Alive
    }
}

// ── RuleSet ───────────────────────────────────────────────────────────────────

/// Birth/Survival rule in the B/S notation.
///
/// - `birth[n]`   = true → a dead cell with exactly `n` alive neighbours is born
/// - `survival[n]` = true → a living cell with exactly `n` alive neighbours survives
#[derive(Debug, Clone)]
pub struct RuleSet {
    /// birth[n] for n in 0..=8
    pub birth: Vec<u8>,
    /// survival[n] for n in 0..=8
    pub survival: Vec<u8>,
}

impl RuleSet {
    fn birth_set(counts: &[u8]) -> Vec<u8> {
        counts.to_vec()
    }

    /// Conway's Game of Life — B3/S23
    pub fn conway() -> Self {
        RuleSet {
            birth: Self::birth_set(&[3]),
            survival: Self::birth_set(&[2, 3]),
        }
    }

    /// HighLife — B36/S23
    pub fn high_life() -> Self {
        RuleSet {
            birth: Self::birth_set(&[3, 6]),
            survival: Self::birth_set(&[2, 3]),
        }
    }

    /// Day and Night — B3678/S34678
    pub fn day_and_night() -> Self {
        RuleSet {
            birth: Self::birth_set(&[3, 6, 7, 8]),
            survival: Self::birth_set(&[3, 4, 6, 7, 8]),
        }
    }

    /// Seeds — B2/S (nothing survives)
    pub fn seeds() -> Self {
        RuleSet {
            birth: Self::birth_set(&[2]),
            survival: vec![],
        }
    }

    /// Mazectric — B3/S1234
    pub fn mazectric() -> Self {
        RuleSet {
            birth: Self::birth_set(&[3]),
            survival: Self::birth_set(&[1, 2, 3, 4]),
        }
    }

    fn births(&self, n: u8) -> bool {
        self.birth.contains(&n)
    }

    fn survives(&self, n: u8) -> bool {
        self.survival.contains(&n)
    }
}

// ── LCG helper ────────────────────────────────────────────────────────────────

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg { state: seed.wrapping_add(1) }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self.state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.state >> 32) as f64) / (u32::MAX as f64 + 1.0)
    }
}

// ── CaGrid ────────────────────────────────────────────────────────────────────

/// A 2D cellular automaton grid.
#[derive(Debug, Clone)]
pub struct CaGrid {
    pub width: usize,
    pub height: usize,
    /// Row-major flat storage: cells[y * width + x]
    pub cells: Vec<CellState>,
    /// Whether to wrap at edges (toroidal topology) instead of treating outside as dead.
    pub wrap: bool,
}

impl CaGrid {
    /// Create an all-dead grid.
    pub fn new(width: usize, height: usize, wrap: bool) -> Self {
        CaGrid {
            width,
            height,
            cells: vec![CellState::Dead; width * height],
            wrap,
        }
    }

    /// Fill cells randomly using an LCG.
    ///
    /// `density` is the fraction of cells that will be alive (0.0–1.0).
    pub fn randomize(&mut self, density: f64, seed: u64) {
        let density = density.clamp(0.0, 1.0);
        let mut lcg = Lcg::new(seed);
        for cell in self.cells.iter_mut() {
            *cell = if lcg.next_f64() < density {
                CellState::Alive
            } else {
                CellState::Dead
            };
        }
    }

    /// Set the cell at `(x, y)`.
    pub fn set(&mut self, x: usize, y: usize, state: CellState) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = state;
        }
    }

    /// Get the cell at `(x, y)`.
    ///
    /// Returns `Dead` if out-of-bounds when `wrap == false`; wraps otherwise.
    pub fn get(&self, x: usize, y: usize) -> CellState {
        if self.width == 0 || self.height == 0 {
            return CellState::Dead;
        }
        if self.wrap {
            let wx = x % self.width;
            let wy = y % self.height;
            self.cells[wy * self.width + wx]
        } else {
            if x < self.width && y < self.height {
                self.cells[y * self.width + x]
            } else {
                CellState::Dead
            }
        }
    }

    /// Count alive Moore (8-connected) neighbours of cell `(x, y)`.
    pub fn neighbor_count(&self, x: usize, y: usize) -> u8 {
        let mut count = 0u8;
        let (xi, yi) = (x as isize, y as isize);
        let (w, h) = (self.width as isize, self.height as isize);

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (xi + dx, yi + dy);
                let state = if self.wrap {
                    let wx = ((nx % w) + w) as usize % self.width;
                    let wy = ((ny % h) + h) as usize % self.height;
                    self.cells[wy * self.width + wx]
                } else if nx >= 0 && nx < w && ny >= 0 && ny < h {
                    self.cells[ny as usize * self.width + nx as usize]
                } else {
                    CellState::Dead
                };
                if state.is_alive() {
                    count += 1;
                }
            }
        }
        count
    }

    /// Compute the next generation according to `rules`.
    pub fn step(&self, rules: &RuleSet) -> CaGrid {
        let mut next = CaGrid::new(self.width, self.height, self.wrap);
        for y in 0..self.height {
            for x in 0..self.width {
                let n = self.neighbor_count(x, y);
                let alive = self.get(x, y).is_alive();
                let new_state = if alive {
                    if rules.survives(n) { CellState::Alive } else { CellState::Dead }
                } else {
                    if rules.births(n) { CellState::Alive } else { CellState::Dead }
                };
                next.set(x, y, new_state);
            }
        }
        next
    }

    /// Count the number of alive cells.
    pub fn alive_count(&self) -> usize {
        self.cells.iter().filter(|&&c| c.is_alive()).count()
    }

    /// Fraction of cells that are alive.
    pub fn density(&self) -> f64 {
        if self.cells.is_empty() {
            return 0.0;
        }
        self.alive_count() as f64 / self.cells.len() as f64
    }

    /// Run the automaton for `generations` steps, collecting all grid states.
    pub fn run(&self, rules: &RuleSet, generations: usize) -> Vec<CaGrid> {
        let mut history = Vec::with_capacity(generations + 1);
        history.push(self.clone());
        let mut current = self.clone();
        for _ in 0..generations {
            let next = current.step(rules);
            history.push(next.clone());
            current = next;
        }
        history
    }

    /// Render as a flat RGB `u8` vector (row-major, 3 bytes per pixel).
    pub fn to_rgb(&self, alive_color: (u8, u8, u8), dead_color: (u8, u8, u8)) -> Vec<u8> {
        let mut buf = Vec::with_capacity(self.width * self.height * 3);
        for &cell in &self.cells {
            let (r, g, b) = if cell.is_alive() { alive_color } else { dead_color };
            buf.push(r);
            buf.push(g);
            buf.push(b);
        }
        buf
    }

    /// Render as ASCII art: `#` for alive, `.` for dead, `\n` between rows.
    pub fn to_ascii(&self) -> String {
        let mut s = String::with_capacity(self.height * (self.width + 1));
        for y in 0..self.height {
            for x in 0..self.width {
                s.push(if self.get(x, y).is_alive() { '#' } else { '.' });
            }
            if y + 1 < self.height {
                s.push('\n');
            }
        }
        s
    }
}

// ── GliderLibrary ─────────────────────────────────────────────────────────────

/// Common patterns expressed as lists of alive-cell relative offsets `(dx, dy)`.
pub struct GliderLibrary;

impl GliderLibrary {
    /// Classic 5-cell glider (moves diagonally in Conway).
    ///
    /// ```text
    ///  .#.
    ///  ..#
    ///  ###
    /// ```
    pub fn glider() -> Vec<(usize, usize)> {
        vec![(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)]
    }

    /// Horizontal 3-cell blinker (period-2 oscillator).
    pub fn blinker() -> Vec<(usize, usize)> {
        vec![(0, 1), (1, 1), (2, 1)]
    }

    /// 2×2 stable block.
    pub fn block() -> Vec<(usize, usize)> {
        vec![(0, 0), (1, 0), (0, 1), (1, 1)]
    }

    /// Place a pattern on the grid at the given top-left position `(x, y)`.
    pub fn place_pattern(grid: &mut CaGrid, pattern: &[(usize, usize)], x: usize, y: usize) {
        for &(dx, dy) in pattern {
            let px = x + dx;
            let py = y + dy;
            if px < grid.width && py < grid.height {
                grid.set(px, py, CellState::Alive);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_grid_stays_empty() {
        let grid = CaGrid::new(10, 10, false);
        let rules = RuleSet::conway();
        let next = grid.step(&rules);
        assert_eq!(next.alive_count(), 0);
    }

    #[test]
    fn block_is_stable_in_conway() {
        let mut grid = CaGrid::new(6, 6, false);
        GliderLibrary::place_pattern(&mut grid, &GliderLibrary::block(), 1, 1);
        let rules = RuleSet::conway();
        let next = grid.step(&rules);
        assert_eq!(grid.alive_count(), next.alive_count(), "Block should be stable");
        // Same positions alive
        for y in 0..6 {
            for x in 0..6 {
                assert_eq!(grid.get(x, y), next.get(x, y), "cell ({},{}) changed", x, y);
            }
        }
    }

    #[test]
    fn blinker_oscillates() {
        let mut grid = CaGrid::new(5, 5, false);
        GliderLibrary::place_pattern(&mut grid, &GliderLibrary::blinker(), 1, 2);
        let rules = RuleSet::conway();
        let gen1 = grid.step(&rules);
        let gen2 = gen1.step(&rules);
        // After 2 steps it should return to original configuration
        assert_eq!(grid.alive_count(), gen2.alive_count());
    }

    #[test]
    fn glider_moves_after_four_steps() {
        let mut grid = CaGrid::new(20, 20, false);
        GliderLibrary::place_pattern(&mut grid, &GliderLibrary::glider(), 1, 1);
        let rules = RuleSet::conway();
        let history = grid.run(&rules, 4);
        // After 4 steps the glider has moved; alive count stays at 5
        assert_eq!(history[0].alive_count(), 5);
        assert_eq!(history[4].alive_count(), 5);
        // At least one cell differs between step 0 and step 4
        let differs = (0..20).any(|y| (0..20).any(|x| history[0].get(x, y) != history[4].get(x, y)));
        assert!(differs, "Glider should have moved after 4 steps");
    }

    #[test]
    fn density_in_range() {
        let mut grid = CaGrid::new(20, 20, false);
        grid.randomize(0.5, 12345);
        let d = grid.density();
        assert!(d >= 0.0 && d <= 1.0, "density out of range: {}", d);
    }

    #[test]
    fn density_zero_for_empty() {
        let grid = CaGrid::new(10, 10, false);
        assert_eq!(grid.density(), 0.0);
    }

    #[test]
    fn to_rgb_correct_length() {
        let grid = CaGrid::new(4, 4, false);
        let rgb = grid.to_rgb((255, 255, 255), (0, 0, 0));
        assert_eq!(rgb.len(), 4 * 4 * 3);
    }

    #[test]
    fn to_ascii_correct_format() {
        let mut grid = CaGrid::new(3, 2, false);
        grid.set(0, 0, CellState::Alive);
        let ascii = grid.to_ascii();
        let lines: Vec<&str> = ascii.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "#..");
        assert_eq!(lines[1], "...");
    }

    #[test]
    fn ruleset_bs_notation_correct() {
        let conway = RuleSet::conway();
        assert!(conway.births(3));
        assert!(!conway.births(2));
        assert!(conway.survives(2));
        assert!(conway.survives(3));
        assert!(!conway.survives(4));

        let hl = RuleSet::high_life();
        assert!(hl.births(6));
        assert!(!hl.births(4));

        let seeds = RuleSet::seeds();
        assert!(seeds.births(2));
        assert!(!seeds.survives(2)); // nothing survives
    }

    #[test]
    fn wrap_get_works() {
        let mut grid = CaGrid::new(5, 5, true);
        grid.set(0, 0, CellState::Alive);
        // With wrap, accessing (5, 5) should give (0, 0)
        assert_eq!(grid.get(5, 5), CellState::Alive);
    }

    #[test]
    fn neighbor_count_corners() {
        let mut grid = CaGrid::new(3, 3, false);
        // Fill all cells alive
        for y in 0..3 { for x in 0..3 { grid.set(x, y, CellState::Alive); } }
        // Corner cell (0,0): 3 alive neighbours (1,0), (0,1), (1,1)
        assert_eq!(grid.neighbor_count(0, 0), 3);
        // Centre cell (1,1): 8 alive neighbours
        assert_eq!(grid.neighbor_count(1, 1), 8);
    }
}
