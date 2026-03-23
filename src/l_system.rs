//! # L-System (Lindenmayer System) Turtle Graphics
//!
//! Generates fractal line patterns via string-rewriting production rules and
//! renders them via turtle-graphics into line-segment lists or rasterised
//! pixel buffers.

use std::collections::{HashMap, HashSet};

// ── LSystem ───────────────────────────────────────────────────────────────────

/// An L-system defined by an axiom and a set of character production rules.
#[derive(Debug, Clone)]
pub struct LSystem {
    /// Starting string (seed).
    pub axiom: String,
    /// Production rules: each variable character maps to its replacement.
    pub rules: HashMap<char, String>,
    /// Characters that are rewritten by production rules.
    pub variables: HashSet<char>,
    /// Characters that pass through unchanged.
    pub constants: HashSet<char>,
}

impl LSystem {
    /// Create an L-system with just an axiom and no rules yet.
    pub fn new(axiom: &str) -> Self {
        Self {
            axiom: axiom.to_string(),
            rules: HashMap::new(),
            variables: HashSet::new(),
            constants: HashSet::new(),
        }
    }

    /// Builder: add a production rule `from → to` and register `from` as a
    /// variable.
    pub fn rule(mut self, from: char, to: &str) -> Self {
        self.rules.insert(from, to.to_string());
        self.variables.insert(from);
        self
    }

    /// Expand the axiom by applying production rules `iterations` times.
    pub fn expand(&self, iterations: usize) -> String {
        let mut current = self.axiom.clone();
        for _ in 0..iterations {
            let mut next = String::new();
            for c in current.chars() {
                if let Some(replacement) = self.rules.get(&c) {
                    next.push_str(replacement);
                } else {
                    next.push(c);
                }
            }
            current = next;
        }
        current
    }

    // ── Named L-systems ───────────────────────────────────────────────────────

    /// Dragon curve: axiom "F", rules F→F+G, G→F-G; angle=90°.
    pub fn dragon_curve() -> Self {
        Self::new("F")
            .rule('F', "F+G")
            .rule('G', "F-G")
    }

    /// Sierpinski triangle: axiom "F-G-G", rules F→F-G+F+G-F, G→GG; angle=120°.
    pub fn sierpinski_triangle() -> Self {
        Self::new("F-G-G")
            .rule('F', "F-G+F+G-F")
            .rule('G', "GG")
    }

    /// Fractal plant: axiom "X", rules X→F+[[X]-X]-F[-FX]+X, F→FF; angle=25°.
    pub fn plant() -> Self {
        Self::new("X")
            .rule('X', "F+[[X]-X]-F[-FX]+X")
            .rule('F', "FF")
    }

    /// Hilbert curve: axiom "A", rules A→+BF-AFA-FB+, B→-AF+BFB+FA-; angle=90°.
    pub fn hilbert_curve() -> Self {
        Self::new("A")
            .rule('A', "+BF-AFA-FB+")
            .rule('B', "-AF+BFB+FA-")
    }
}

// ── TurtleState ───────────────────────────────────────────────────────────────

/// The turtle's current drawing state.
#[derive(Debug, Clone)]
pub struct TurtleState {
    pub x: f64,
    pub y: f64,
    /// Current heading in degrees (0° = right, 90° = up).
    pub angle_deg: f64,
}

// ── TurtleRenderer ────────────────────────────────────────────────────────────

/// Renders an L-system instruction string into a list of line segments.
pub struct TurtleRenderer {
    /// Length of a single forward step in world units.
    pub step_size: f64,
    /// Angle turned on `+` / `-` commands in degrees.
    pub angle_delta_deg: f64,
    /// Canvas width (for normalisation).
    pub canvas_w: usize,
    /// Canvas height (for normalisation).
    pub canvas_h: usize,
}

impl TurtleRenderer {
    /// Execute turtle-graphics commands and return `(x1, y1, x2, y2)` line
    /// segments for every forward-draw step.
    ///
    /// Commands:
    /// - `F` / `G` — move forward, emit segment.
    /// - `+` — turn left by `angle_delta_deg`.
    /// - `-` — turn right by `angle_delta_deg`.
    /// - `[` — push state.
    /// - `]` — pop state.
    /// - anything else — ignored.
    pub fn render(
        &self,
        instructions: &str,
        start: TurtleState,
    ) -> Vec<(f64, f64, f64, f64)> {
        let mut state = start;
        let mut stack: Vec<TurtleState> = Vec::new();
        let mut segments: Vec<(f64, f64, f64, f64)> = Vec::new();

        for ch in instructions.chars() {
            match ch {
                'F' | 'G' => {
                    let rad = state.angle_deg.to_radians();
                    let x2 = state.x + self.step_size * rad.cos();
                    let y2 = state.y + self.step_size * rad.sin();
                    segments.push((state.x, state.y, x2, y2));
                    state.x = x2;
                    state.y = y2;
                }
                '+' => state.angle_deg += self.angle_delta_deg,
                '-' => state.angle_deg -= self.angle_delta_deg,
                '[' => stack.push(state.clone()),
                ']' => {
                    if let Some(saved) = stack.pop() {
                        state = saved;
                    }
                }
                _ => {}
            }
        }
        segments
    }

    /// Translate and uniformly scale `segments` to fit within `(w, h)`.
    pub fn normalize_segments(
        segments: &mut Vec<(f64, f64, f64, f64)>,
        w: usize,
        h: usize,
    ) {
        if segments.is_empty() {
            return;
        }
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for &(x1, y1, x2, y2) in segments.iter() {
            min_x = min_x.min(x1).min(x2);
            min_y = min_y.min(y1).min(y2);
            max_x = max_x.max(x1).max(x2);
            max_y = max_y.max(y1).max(y2);
        }

        let range_x = (max_x - min_x).max(1e-12);
        let range_y = (max_y - min_y).max(1e-12);
        let margin = 4.0;
        let scale = ((w as f64 - margin * 2.0) / range_x)
            .min((h as f64 - margin * 2.0) / range_y);

        for seg in segments.iter_mut() {
            seg.0 = (seg.0 - min_x) * scale + margin;
            seg.1 = (seg.1 - min_y) * scale + margin;
            seg.2 = (seg.2 - min_x) * scale + margin;
            seg.3 = (seg.3 - min_y) * scale + margin;
        }
    }
}

// ── Rasterisation ─────────────────────────────────────────────────────────────

/// Rasterise `segments` onto an RGB byte buffer of `w × h` pixels using
/// Bresenham's line algorithm.
///
/// Returns a `w * h * 3` byte buffer (R, G, B interleaved), initialised to
/// black with segments drawn in `color`.
pub fn to_pixels(
    segments: &[(f64, f64, f64, f64)],
    w: usize,
    h: usize,
    color: (u8, u8, u8),
) -> Vec<u8> {
    let mut buf = vec![0u8; w * h * 3];

    let set_pixel = |buf: &mut Vec<u8>, x: i64, y: i64| {
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let idx = ((y as usize) * w + (x as usize)) * 3;
            buf[idx]     = color.0;
            buf[idx + 1] = color.1;
            buf[idx + 2] = color.2;
        }
    };

    for &(fx1, fy1, fx2, fy2) in segments {
        let mut x0 = fx1.round() as i64;
        let mut y0 = fy1.round() as i64;
        let x1 = fx2.round() as i64;
        let y1 = fy2.round() as i64;

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx: i64 = if x0 < x1 { 1 } else { -1 };
        let sy: i64 = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            set_pixel(&mut buf, x0, y0);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }
    buf
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dragon_curve_expand_one_iteration() {
        let ls = LSystem::dragon_curve();
        let expanded = ls.expand(1);
        // F → F+G
        assert_eq!(expanded, "F+G");
    }

    #[test]
    fn dragon_curve_expand_two_iterations() {
        let ls = LSystem::dragon_curve();
        let expanded = ls.expand(2);
        // F+G → (F+G)+(F-G) = F+G+F-G
        assert_eq!(expanded, "F+G+F-G");
    }

    #[test]
    fn plant_segments_nonempty() {
        let ls = LSystem::plant();
        let instructions = ls.expand(3);
        let renderer = TurtleRenderer {
            step_size: 5.0,
            angle_delta_deg: 25.0,
            canvas_w: 512,
            canvas_h: 512,
        };
        let start = TurtleState { x: 256.0, y: 10.0, angle_deg: 90.0 };
        let segments = renderer.render(&instructions, start);
        assert!(!segments.is_empty(), "plant should produce segments");
    }

    #[test]
    fn normalize_keeps_within_bounds() {
        let mut segments = vec![
            (0.0_f64, 0.0_f64, 1000.0_f64, 1000.0_f64),
            (-500.0, -500.0, 500.0, 500.0),
        ];
        TurtleRenderer::normalize_segments(&mut segments, 800, 600);
        for &(x1, y1, x2, y2) in &segments {
            assert!(x1 >= 0.0 && x1 <= 800.0, "x1 out of bounds: {x1}");
            assert!(y1 >= 0.0 && y1 <= 600.0, "y1 out of bounds: {y1}");
            assert!(x2 >= 0.0 && x2 <= 800.0, "x2 out of bounds: {x2}");
            assert!(y2 >= 0.0 && y2 <= 600.0, "y2 out of bounds: {y2}");
        }
    }

    #[test]
    fn to_pixels_correct_buffer_size() {
        let segs = vec![(0.0_f64, 0.0_f64, 10.0_f64, 10.0_f64)];
        let buf = to_pixels(&segs, 64, 64, (255, 0, 0));
        assert_eq!(buf.len(), 64 * 64 * 3);
    }

    #[test]
    fn to_pixels_draws_something() {
        let segs = vec![(10.0_f64, 10.0_f64, 50.0_f64, 10.0_f64)];
        let buf = to_pixels(&segs, 64, 64, (255, 255, 255));
        let lit = buf.chunks(3).any(|px| px[0] > 0);
        assert!(lit, "expected at least one non-black pixel");
    }

    #[test]
    fn hilbert_curve_expands_without_panic() {
        let ls = LSystem::hilbert_curve();
        let s = ls.expand(4);
        assert!(!s.is_empty());
    }
}
