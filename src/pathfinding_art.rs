//! Pathfinding as an artistic medium — maze generation and solving.
//!
//! Supports three maze-generation algorithms (recursive backtracker, Prim's,
//! Sidewinder) and two solvers (A* and BFS).  Both an RGB pixel renderer and
//! an ASCII renderer are provided.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Cell / Maze
// ---------------------------------------------------------------------------

/// Direction indices: 0=North, 1=East, 2=South, 3=West.
const N: usize = 0;
const E: usize = 1;
const S: usize = 2;
const W: usize = 3;

/// A single cell in the maze grid.
#[derive(Debug, Clone)]
pub struct Cell {
    pub x: u32,
    pub y: u32,
    /// Walls present on each side: \[N, E, S, W\].
    pub walls: [bool; 4],
}

impl Cell {
    fn new(x: u32, y: u32) -> Self {
        Self {
            x,
            y,
            walls: [true; 4],
        }
    }
}

/// A rectangular grid of cells forming a maze.
#[derive(Debug, Clone)]
pub struct Maze {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<Vec<Cell>>,
}

impl Maze {
    /// Create a new maze with all walls intact.
    pub fn new(width: u32, height: u32) -> Self {
        let cells = (0..height)
            .map(|y| (0..width).map(|x| Cell::new(x, y)).collect())
            .collect();
        Self {
            width,
            height,
            cells,
        }
    }

    /// Remove the wall on `direction` of cell `(x, y)` and the matching
    /// wall on the neighbour.
    pub fn remove_wall(&mut self, x: u32, y: u32, direction: usize) {
        self.cells[y as usize][x as usize].walls[direction] = false;
        match direction {
            N if y > 0 => self.cells[(y - 1) as usize][x as usize].walls[S] = false,
            S if y + 1 < self.height => self.cells[(y + 1) as usize][x as usize].walls[N] = false,
            E if x + 1 < self.width => self.cells[y as usize][(x + 1) as usize].walls[W] = false,
            W if x > 0 => self.cells[y as usize][(x - 1) as usize].walls[E] = false,
            _ => {}
        }
    }

    /// Query whether a wall is present.
    pub fn has_wall(&self, x: u32, y: u32, dir: usize) -> bool {
        self.cells[y as usize][x as usize].walls[dir]
    }

    /// Return neighbours reachable from `(x, y)` (no wall between them).
    fn neighbours(&self, x: u32, y: u32) -> Vec<(u32, u32)> {
        let mut result = Vec::new();
        if y > 0 && !self.has_wall(x, y, N) {
            result.push((x, y - 1));
        }
        if x + 1 < self.width && !self.has_wall(x, y, E) {
            result.push((x + 1, y));
        }
        if y + 1 < self.height && !self.has_wall(x, y, S) {
            result.push((x, y + 1));
        }
        if x > 0 && !self.has_wall(x, y, W) {
            result.push((x - 1, y));
        }
        result
    }

    /// Return all four directional neighbours that exist (regardless of walls).
    fn all_neighbours(&self, x: u32, y: u32) -> Vec<(u32, u32, usize)> {
        let mut result = Vec::new();
        if y > 0 {
            result.push((x, y - 1, N));
        }
        if x + 1 < self.width {
            result.push((x + 1, y, E));
        }
        if y + 1 < self.height {
            result.push((x, y + 1, S));
        }
        if x > 0 {
            result.push((x - 1, y, W));
        }
        result
    }
}

// ---------------------------------------------------------------------------
// MazeGenerator
// ---------------------------------------------------------------------------

/// Maze generation algorithms.
#[derive(Debug, Default)]
pub struct MazeGenerator;

impl MazeGenerator {
    /// Recursive backtracker (DFS) — produces long winding corridors.
    pub fn recursive_backtracker(width: u32, height: u32, seed: u64) -> Maze {
        let mut maze = Maze::new(width, height);
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut rng = LcgRng::new(seed);

        let start = (0u32, 0u32);
        visited.insert(start);
        stack.push(start);

        while let Some(&(x, y)) = stack.last() {
            let unvisited: Vec<(u32, u32, usize)> = maze
                .all_neighbours(x, y)
                .into_iter()
                .filter(|(nx, ny, _)| !visited.contains(&(*nx, *ny)))
                .collect();

            if unvisited.is_empty() {
                stack.pop();
            } else {
                let idx = rng.next_usize() % unvisited.len();
                let (nx, ny, dir) = unvisited[idx];
                maze.remove_wall(x, y, dir);
                visited.insert((nx, ny));
                stack.push((nx, ny));
            }
        }
        maze
    }

    /// Prim's algorithm — produces mazes with many short dead ends.
    pub fn prims_algorithm(width: u32, height: u32, seed: u64) -> Maze {
        let mut maze = Maze::new(width, height);
        let mut in_maze = HashSet::new();
        let mut frontier: Vec<(u32, u32, u32, u32, usize)> = Vec::new(); // (nx, ny, from_x, from_y, dir)
        let mut rng = LcgRng::new(seed);

        let sx = rng.next_usize() as u32 % width;
        let sy = rng.next_usize() as u32 % height;
        in_maze.insert((sx, sy));
        for (nx, ny, dir) in maze.all_neighbours(sx, sy) {
            frontier.push((nx, ny, sx, sy, dir));
        }

        while !frontier.is_empty() {
            let idx = rng.next_usize() % frontier.len();
            let (nx, ny, fx, fy, dir) = frontier.swap_remove(idx);

            if in_maze.contains(&(nx, ny)) {
                continue;
            }
            maze.remove_wall(fx, fy, dir);
            in_maze.insert((nx, ny));

            for (nnx, nny, ndir) in maze.all_neighbours(nx, ny) {
                if !in_maze.contains(&(nnx, nny)) {
                    frontier.push((nnx, nny, nx, ny, ndir));
                }
            }
        }
        maze
    }

    /// Sidewinder algorithm — produces mazes with a strong horizontal bias.
    pub fn sidewinder(width: u32, height: u32, seed: u64) -> Maze {
        let mut maze = Maze::new(width, height);
        let mut rng = LcgRng::new(seed);

        // Top row: open all east walls except last
        for x in 0..width - 1 {
            maze.remove_wall(x, 0, E);
        }

        for y in 1..height {
            let mut run_start = 0u32;
            for x in 0..width {
                let at_east_boundary = x + 1 >= width;
                let carve_east = !at_east_boundary && rng.next_bool();

                if carve_east {
                    maze.remove_wall(x, y, E);
                } else {
                    // Carve north from a random cell in the current run
                    let run_len = x - run_start + 1;
                    let member = run_start + (rng.next_usize() as u32 % run_len);
                    maze.remove_wall(member, y, N);
                    run_start = x + 1;
                }
            }
        }
        maze
    }
}

// ---------------------------------------------------------------------------
// PathNode (for A*)
// ---------------------------------------------------------------------------

/// A node in the A* open set.
#[derive(Debug, Clone)]
pub struct PathNode {
    pub x: u32,
    pub y: u32,
    pub g: f64,
    pub h: f64,
    pub f: f64,
}

impl PartialEq for PathNode {
    fn eq(&self, other: &Self) -> bool {
        (self.x, self.y) == (other.x, other.y)
    }
}
impl Eq for PathNode {}

impl PartialOrd for PathNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PathNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap by f score
        other.f.partial_cmp(&self.f).unwrap_or(Ordering::Equal)
    }
}

// ---------------------------------------------------------------------------
// PathFinder
// ---------------------------------------------------------------------------

/// Maze-solving algorithms.
#[derive(Debug, Default)]
pub struct PathFinder;

impl PathFinder {
    /// A* search through the maze.
    pub fn astar(maze: &Maze, start: (u32, u32), end: (u32, u32)) -> Option<Vec<(u32, u32)>> {
        let h = |x: u32, y: u32| Self::manhattan((x, y), end);

        let mut open: BinaryHeap<PathNode> = BinaryHeap::new();
        let mut came_from: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
        let mut g_score: HashMap<(u32, u32), f64> = HashMap::new();

        g_score.insert(start, 0.0);
        open.push(PathNode {
            x: start.0,
            y: start.1,
            g: 0.0,
            h: h(start.0, start.1),
            f: h(start.0, start.1),
        });

        while let Some(current) = open.pop() {
            let pos = (current.x, current.y);
            if pos == end {
                return Some(Self::reconstruct_path(&came_from, end));
            }
            for (nx, ny) in maze.neighbours(current.x, current.y) {
                let tentative_g = g_score[&pos] + 1.0;
                if tentative_g < *g_score.get(&(nx, ny)).unwrap_or(&f64::INFINITY) {
                    came_from.insert((nx, ny), pos);
                    g_score.insert((nx, ny), tentative_g);
                    let h_val = h(nx, ny);
                    open.push(PathNode {
                        x: nx,
                        y: ny,
                        g: tentative_g,
                        h: h_val,
                        f: tentative_g + h_val,
                    });
                }
            }
        }
        None
    }

    /// Breadth-first search through the maze.
    pub fn bfs(maze: &Maze, start: (u32, u32), end: (u32, u32)) -> Option<Vec<(u32, u32)>> {
        let mut visited = HashSet::new();
        let mut came_from: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
        let mut queue = VecDeque::new();

        visited.insert(start);
        queue.push_back(start);

        while let Some(pos) = queue.pop_front() {
            if pos == end {
                return Some(Self::reconstruct_path(&came_from, end));
            }
            for neighbour in maze.neighbours(pos.0, pos.1) {
                if !visited.contains(&neighbour) {
                    visited.insert(neighbour);
                    came_from.insert(neighbour, pos);
                    queue.push_back(neighbour);
                }
            }
        }
        None
    }

    /// Manhattan distance heuristic.
    pub fn manhattan(a: (u32, u32), b: (u32, u32)) -> f64 {
        let dx = (a.0 as i64 - b.0 as i64).unsigned_abs() as f64;
        let dy = (a.1 as i64 - b.1 as i64).unsigned_abs() as f64;
        dx + dy
    }

    fn reconstruct_path(
        came_from: &HashMap<(u32, u32), (u32, u32)>,
        mut current: (u32, u32),
    ) -> Vec<(u32, u32)> {
        let mut path = vec![current];
        while let Some(&prev) = came_from.get(&current) {
            path.push(prev);
            current = prev;
        }
        path.reverse();
        path
    }
}

// ---------------------------------------------------------------------------
// MazeRenderer
// ---------------------------------------------------------------------------

/// Renders mazes to pixel buffers or ASCII strings.
#[derive(Debug, Default)]
pub struct MazeRenderer;

impl MazeRenderer {
    /// Render to an RGB pixel grid.
    ///
    /// Each maze cell occupies `cell_px × cell_px` pixels.
    /// Walls are black, path cells show a rainbow gradient, empty cells white.
    pub fn render_maze(
        maze: &Maze,
        path: Option<&[(u32, u32)]>,
        img_width: u32,
        img_height: u32,
    ) -> Vec<Vec<[u8; 3]>> {
        let cell_w = (img_width / maze.width).max(1);
        let cell_h = (img_height / maze.height).max(1);
        let wall_thickness = (cell_w / 6).max(1);

        let path_set: HashMap<(u32, u32), usize> = path
            .unwrap_or(&[])
            .iter()
            .enumerate()
            .map(|(i, &p)| (p, i))
            .collect();
        let path_len = path.map(|p| p.len()).unwrap_or(0);

        let mut pixels: Vec<Vec<[u8; 3]>> =
            vec![vec![[255u8, 255, 255]; img_width as usize]; img_height as usize];

        for y in 0..maze.height {
            for x in 0..maze.width {
                let px = x * cell_w;
                let py = y * cell_h;

                // Fill cell interior
                let color: [u8; 3] = if let Some(&idx) = path_set.get(&(x, y)) {
                    Self::color_path(&[(x, y)], path_len, idx)
                } else {
                    [240, 240, 240]
                };

                for dy in 0..cell_h {
                    for dx in 0..cell_w {
                        let px2 = (px + dx) as usize;
                        let py2 = (py + dy) as usize;
                        if py2 < pixels.len() && px2 < pixels[0].len() {
                            pixels[py2][px2] = color;
                        }
                    }
                }

                // Draw walls
                let cell = &maze.cells[y as usize][x as usize];
                // North wall
                if cell.walls[N] {
                    for dx in 0..cell_w {
                        for t in 0..wall_thickness {
                            let px2 = (px + dx) as usize;
                            let py2 = (py + t) as usize;
                            if py2 < pixels.len() && px2 < pixels[0].len() {
                                pixels[py2][px2] = [0, 0, 0];
                            }
                        }
                    }
                }
                // West wall
                if cell.walls[W] {
                    for dy in 0..cell_h {
                        for t in 0..wall_thickness {
                            let px2 = (px + t) as usize;
                            let py2 = (py + dy) as usize;
                            if py2 < pixels.len() && px2 < pixels[0].len() {
                                pixels[py2][px2] = [0, 0, 0];
                            }
                        }
                    }
                }
                // South wall (only for bottom row)
                if cell.walls[S] && y + 1 == maze.height {
                    for dx in 0..cell_w {
                        for t in 0..wall_thickness {
                            let px2 = (px + dx) as usize;
                            let py2 = (py + cell_h - 1 - t) as usize;
                            if py2 < pixels.len() && px2 < pixels[0].len() {
                                pixels[py2][px2] = [0, 0, 0];
                            }
                        }
                    }
                }
                // East wall (only for rightmost column)
                if cell.walls[E] && x + 1 == maze.width {
                    for dy in 0..cell_h {
                        for t in 0..wall_thickness {
                            let px2 = (px + cell_w - 1 - t) as usize;
                            let py2 = (py + dy) as usize;
                            if py2 < pixels.len() && px2 < pixels[0].len() {
                                pixels[py2][px2] = [0, 0, 0];
                            }
                        }
                    }
                }
            }
        }
        pixels
    }

    /// Render the maze to an ASCII string.
    pub fn render_ascii(maze: &Maze, path: Option<&[(u32, u32)]>) -> String {
        let path_set: HashSet<(u32, u32)> = path.unwrap_or(&[]).iter().copied().collect();

        let mut lines = Vec::new();
        // Top border
        let top: String = (0..maze.width).map(|_| "+---").collect::<String>() + "+";
        lines.push(top);

        for y in 0..maze.height {
            // Row of cells
            let mut row = String::from("|");
            for x in 0..maze.width {
                let symbol = if path_set.contains(&(x, y)) {
                    " * "
                } else {
                    "   "
                };
                row.push_str(symbol);
                if maze.has_wall(x, y, E) {
                    row.push('|');
                } else {
                    row.push(' ');
                }
            }
            lines.push(row);

            // Horizontal separators
            let mut sep = String::new();
            for x in 0..maze.width {
                sep.push('+');
                if maze.has_wall(x, y, S) {
                    sep.push_str("---");
                } else {
                    sep.push_str("   ");
                }
            }
            sep.push('+');
            lines.push(sep);
        }

        lines.join("\n")
    }

    /// Compute a rainbow gradient colour for position `idx` out of `total`.
    pub fn color_path(_path: &[(u32, u32)], total: usize, idx: usize) -> [u8; 3] {
        let t = if total <= 1 {
            0.0
        } else {
            idx as f64 / (total - 1) as f64
        };
        // HSV→RGB where H goes 0→240° (red→blue through green)
        let hue = t * 240.0;
        let (r, g, b) = hsv_to_rgb(hue, 1.0, 1.0);
        [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64) {
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
    (r1 + m, g1 + m, b1 + m)
}

/// Simple linear-congruential PRNG.
struct LcgRng(u64);

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1))
    }
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn next_usize(&mut self) -> usize {
        self.next() as usize
    }
    fn next_bool(&mut self) -> bool {
        self.next() & 1 == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maze_new_all_walls() {
        let maze = Maze::new(5, 5);
        assert!(maze.has_wall(2, 2, N));
        assert!(maze.has_wall(2, 2, E));
    }

    #[test]
    fn test_remove_wall() {
        let mut maze = Maze::new(5, 5);
        maze.remove_wall(0, 0, E);
        assert!(!maze.has_wall(0, 0, E));
        assert!(!maze.has_wall(1, 0, W));
    }

    #[test]
    fn test_recursive_backtracker_connected() {
        let maze = MazeGenerator::recursive_backtracker(10, 10, 42);
        // BFS should be able to reach every cell
        let path = PathFinder::bfs(&maze, (0, 0), (9, 9));
        assert!(path.is_some(), "Maze should be fully connected");
    }

    #[test]
    fn test_prims_connected() {
        let maze = MazeGenerator::prims_algorithm(8, 8, 99);
        let path = PathFinder::bfs(&maze, (0, 0), (7, 7));
        assert!(path.is_some());
    }

    #[test]
    fn test_sidewinder_connected() {
        let maze = MazeGenerator::sidewinder(8, 8, 7);
        let path = PathFinder::bfs(&maze, (0, 0), (7, 7));
        assert!(path.is_some());
    }

    #[test]
    fn test_astar_finds_path() {
        let maze = MazeGenerator::recursive_backtracker(6, 6, 1);
        let path = PathFinder::astar(&maze, (0, 0), (5, 5));
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p[0], (0, 0));
        assert_eq!(*p.last().unwrap(), (5, 5));
    }

    #[test]
    fn test_manhattan() {
        assert_eq!(PathFinder::manhattan((0, 0), (3, 4)), 7.0);
    }

    #[test]
    fn test_ascii_render() {
        let maze = MazeGenerator::recursive_backtracker(4, 4, 5);
        let ascii = MazeRenderer::render_ascii(&maze, None);
        assert!(ascii.contains('+'));
        assert!(ascii.contains('-'));
    }

    #[test]
    fn test_color_path_gradient() {
        let path: Vec<(u32, u32)> = (0..10).map(|i| (i, 0)).collect();
        let c0 = MazeRenderer::color_path(&path, path.len(), 0);
        let c9 = MazeRenderer::color_path(&path, path.len(), 9);
        // Start should be red-ish, end blue-ish
        assert!(c0[0] > c0[2], "Start should be more red than blue");
        assert!(c9[2] > c9[0], "End should be more blue than red");
    }
}
