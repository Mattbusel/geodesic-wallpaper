//! Origami crease-pattern generator.
//!
//! Provides primitives for describing, transforming, and rendering origami
//! crease patterns, together with factory functions for classic bases.

#![allow(dead_code)]

use std::f64::consts::PI;

// ── CreaseType ────────────────────────────────────────────────────────────────

/// The type of a fold line in a crease pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreaseType {
    /// Mountain fold (fold toward viewer).
    Mountain,
    /// Valley fold (fold away from viewer).
    Valley,
    /// Flat / reference crease (no fold).
    Flat,
}

impl CreaseType {
    /// Standard origami diagram symbol.
    pub fn symbol(self) -> char {
        match self {
            CreaseType::Mountain => 'M',
            CreaseType::Valley => 'V',
            CreaseType::Flat => 'F',
        }
    }
}

// ── CreaseLine ────────────────────────────────────────────────────────────────

/// A single line segment in a crease pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct CreaseLine {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub crease_type: CreaseType,
}

impl CreaseLine {
    /// Construct a new crease line.
    pub fn new(x1: f64, y1: f64, x2: f64, y2: f64, crease_type: CreaseType) -> Self {
        CreaseLine {
            x1,
            y1,
            x2,
            y2,
            crease_type,
        }
    }
}

// ── FoldOperation ─────────────────────────────────────────────────────────────

/// A geometric operation applied to all crease lines.
#[derive(Debug, Clone, PartialEq)]
pub enum FoldOperation {
    /// Fold in the given direction: +1 = right/up, -1 = left/down.
    Fold(i8),
    /// Unfold (no-op here — returns pattern unchanged for symmetry).
    Unfold,
    /// Rotate all points around `(cx, cy)` by `angle` radians.
    RotatePoint { cx: f64, cy: f64, angle: f64 },
    /// Mirror all points about the vertical axis (negate x).
    MirrorY,
    /// Mirror all points about the horizontal axis (negate y).
    MirrorX,
}

// ── CreasePattern ─────────────────────────────────────────────────────────────

/// A complete crease pattern for a sheet of paper.
#[derive(Debug, Clone, Default)]
pub struct CreasePattern {
    /// All crease lines.
    pub creases: Vec<CreaseLine>,
    /// Width of the paper in model units.
    pub width: f64,
    /// Height of the paper in model units.
    pub height: f64,
}

impl CreasePattern {
    /// Construct an empty crease pattern for paper of size `width × height`.
    pub fn new(width: f64, height: f64) -> Self {
        CreasePattern {
            creases: Vec::new(),
            width,
            height,
        }
    }

    /// Add a crease line to the pattern.
    pub fn add_crease(&mut self, line: CreaseLine) {
        self.creases.push(line);
    }

    /// Apply a [`FoldOperation`] to every crease line, returning a new pattern.
    pub fn apply_fold(&self, op: &FoldOperation) -> CreasePattern {
        let new_creases = self.creases.iter().map(|c| apply_op(c, op)).collect();
        CreasePattern {
            creases: new_creases,
            width: self.width,
            height: self.height,
        }
    }

    /// Render the crease pattern into a `pixel_height × pixel_width` RGB
    /// pixel buffer (white background).
    ///
    /// Mountain folds → red, Valley folds → blue, Flat → grey.
    pub fn render(&self, pixel_width: u32, pixel_height: u32) -> Vec<Vec<[u8; 3]>> {
        let mut buffer = vec![vec![[255u8; 3]; pixel_width as usize]; pixel_height as usize];

        let scale_x = pixel_width as f64 / self.width.max(1e-9);
        let scale_y = pixel_height as f64 / self.height.max(1e-9);

        for crease in &self.creases {
            let color = match crease.crease_type {
                CreaseType::Mountain => [200u8, 50u8, 50u8],
                CreaseType::Valley => [50u8, 50u8, 200u8],
                CreaseType::Flat => [160u8, 160u8, 160u8],
            };
            draw_line(
                &mut buffer,
                crease.x1 * scale_x,
                crease.y1 * scale_y,
                crease.x2 * scale_x,
                crease.y2 * scale_y,
                color,
                pixel_width,
                pixel_height,
            );
        }
        buffer
    }
}

// ── OrigamiPatterns ───────────────────────────────────────────────────────────

/// Factory for well-known origami crease patterns.
pub struct OrigamiPatterns;

impl OrigamiPatterns {
    // ── Crane base ────────────────────────────────────────────────────────────

    /// Simplified crane base crease pattern on a 1×1 square.
    ///
    /// Includes the preliminary base folds plus the petal-fold diagonals.
    pub fn crane_base() -> CreasePattern {
        let mut cp = CreasePattern::new(1.0, 1.0);

        // Horizontal and vertical centre folds (valley).
        cp.add_crease(CreaseLine::new(0.0, 0.5, 1.0, 0.5, CreaseType::Valley));
        cp.add_crease(CreaseLine::new(0.5, 0.0, 0.5, 1.0, CreaseType::Valley));

        // Diagonal folds (mountain).
        cp.add_crease(CreaseLine::new(0.0, 0.0, 1.0, 1.0, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(1.0, 0.0, 0.0, 1.0, CreaseType::Mountain));

        // Quarter-diagonal petal folds (valley).
        cp.add_crease(CreaseLine::new(0.0, 0.5, 0.5, 0.0, CreaseType::Valley));
        cp.add_crease(CreaseLine::new(0.5, 0.0, 1.0, 0.5, CreaseType::Valley));
        cp.add_crease(CreaseLine::new(1.0, 0.5, 0.5, 1.0, CreaseType::Valley));
        cp.add_crease(CreaseLine::new(0.5, 1.0, 0.0, 0.5, CreaseType::Valley));

        // Inner petal fold diagonals (mountain).
        cp.add_crease(CreaseLine::new(0.5, 0.5, 0.25, 0.25, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(0.5, 0.5, 0.75, 0.25, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(0.5, 0.5, 0.75, 0.75, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(0.5, 0.5, 0.25, 0.75, CreaseType::Mountain));

        cp
    }

    // ── Waterbomb base ────────────────────────────────────────────────────────

    /// Waterbomb base crease pattern on a 1×1 square.
    pub fn waterbomb_base() -> CreasePattern {
        let mut cp = CreasePattern::new(1.0, 1.0);

        // Horizontal centre (mountain) and diagonals (valley).
        cp.add_crease(CreaseLine::new(0.0, 0.5, 1.0, 0.5, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(0.5, 0.0, 0.5, 1.0, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(0.0, 0.0, 1.0, 1.0, CreaseType::Valley));
        cp.add_crease(CreaseLine::new(1.0, 0.0, 0.0, 1.0, CreaseType::Valley));

        cp
    }

    // ── Preliminary base ──────────────────────────────────────────────────────

    /// Preliminary (square) base crease pattern on a 1×1 square.
    pub fn preliminary_base() -> CreasePattern {
        let mut cp = CreasePattern::new(1.0, 1.0);

        // Diagonals (valley).
        cp.add_crease(CreaseLine::new(0.0, 0.0, 1.0, 1.0, CreaseType::Valley));
        cp.add_crease(CreaseLine::new(1.0, 0.0, 0.0, 1.0, CreaseType::Valley));
        // Horizontal and vertical centre (mountain).
        cp.add_crease(CreaseLine::new(0.0, 0.5, 1.0, 0.5, CreaseType::Mountain));
        cp.add_crease(CreaseLine::new(0.5, 0.0, 0.5, 1.0, CreaseType::Mountain));

        cp
    }

    // ── Grid fold ─────────────────────────────────────────────────────────────

    /// Generate an n×m grid of valley fold crease lines.
    ///
    /// Creates `cols − 1` vertical and `rows − 1` horizontal equally-spaced
    /// valley folds on a `width × height` sheet.
    pub fn grid_fold(width: f64, height: f64, rows: u32, cols: u32) -> CreasePattern {
        let mut cp = CreasePattern::new(width, height);

        // Vertical folds.
        for c in 1..cols {
            let x = width * c as f64 / cols as f64;
            cp.add_crease(CreaseLine::new(x, 0.0, x, height, CreaseType::Valley));
        }
        // Horizontal folds.
        for r in 1..rows {
            let y = height * r as f64 / rows as f64;
            cp.add_crease(CreaseLine::new(0.0, y, width, y, CreaseType::Valley));
        }

        cp
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Apply a single [`FoldOperation`] to a [`CreaseLine`].
fn apply_op(line: &CreaseLine, op: &FoldOperation) -> CreaseLine {
    match op {
        FoldOperation::Fold(_) | FoldOperation::Unfold => line.clone(),
        FoldOperation::MirrorY => {
            CreaseLine::new(-line.x1, line.y1, -line.x2, line.y2, line.crease_type)
        }
        FoldOperation::MirrorX => {
            CreaseLine::new(line.x1, -line.y1, line.x2, -line.y2, line.crease_type)
        }
        FoldOperation::RotatePoint { cx, cy, angle } => {
            let (x1, y1) = rotate(*cx, *cy, line.x1, line.y1, *angle);
            let (x2, y2) = rotate(*cx, *cy, line.x2, line.y2, *angle);
            CreaseLine::new(x1, y1, x2, y2, line.crease_type)
        }
    }
}

/// Rotate point `(px, py)` around centre `(cx, cy)` by `angle` radians.
fn rotate(cx: f64, cy: f64, px: f64, py: f64, angle: f64) -> (f64, f64) {
    let dx = px - cx;
    let dy = py - cy;
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    (cx + dx * cos_a - dy * sin_a, cy + dx * sin_a + dy * cos_a)
}

/// Bresenham-ish line drawing onto an RGB pixel buffer.
fn draw_line(
    buffer: &mut Vec<Vec<[u8; 3]>>,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: [u8; 3],
    width: u32,
    height: u32,
) {
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()) as u32 + 1).max(1) * 4;
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let x = (x0 + (x1 - x0) * t) as i64;
        let y = (y0 + (y1 - y0) * t) as i64;
        if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
            buffer[y as usize][x as usize] = color;
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crease_type_symbols() {
        assert_eq!(CreaseType::Mountain.symbol(), 'M');
        assert_eq!(CreaseType::Valley.symbol(), 'V');
        assert_eq!(CreaseType::Flat.symbol(), 'F');
    }

    #[test]
    fn test_crane_base_has_creases() {
        let cp = OrigamiPatterns::crane_base();
        assert!(!cp.creases.is_empty());
    }

    #[test]
    fn test_waterbomb_base_has_creases() {
        let cp = OrigamiPatterns::waterbomb_base();
        assert!(!cp.creases.is_empty());
    }

    #[test]
    fn test_preliminary_base_has_creases() {
        let cp = OrigamiPatterns::preliminary_base();
        assert!(!cp.creases.is_empty());
    }

    #[test]
    fn test_grid_fold_count() {
        let cp = OrigamiPatterns::grid_fold(1.0, 1.0, 4, 4);
        // 3 vertical + 3 horizontal = 6 folds.
        assert_eq!(cp.creases.len(), 6);
    }

    #[test]
    fn test_grid_fold_types_are_valley() {
        let cp = OrigamiPatterns::grid_fold(1.0, 1.0, 3, 3);
        for c in &cp.creases {
            assert_eq!(c.crease_type, CreaseType::Valley);
        }
    }

    #[test]
    fn test_apply_mirror_y() {
        let mut cp = CreasePattern::new(1.0, 1.0);
        cp.add_crease(CreaseLine::new(0.5, 0.0, 0.5, 1.0, CreaseType::Valley));
        let mirrored = cp.apply_fold(&FoldOperation::MirrorY);
        assert!((mirrored.creases[0].x1 - (-0.5)).abs() < 1e-9);
        assert!((mirrored.creases[0].x2 - (-0.5)).abs() < 1e-9);
    }

    #[test]
    fn test_apply_mirror_x() {
        let mut cp = CreasePattern::new(1.0, 1.0);
        cp.add_crease(CreaseLine::new(0.0, 0.3, 1.0, 0.7, CreaseType::Mountain));
        let mirrored = cp.apply_fold(&FoldOperation::MirrorX);
        assert!((mirrored.creases[0].y1 - (-0.3)).abs() < 1e-9);
    }

    #[test]
    fn test_apply_rotate_180() {
        let mut cp = CreasePattern::new(1.0, 1.0);
        cp.add_crease(CreaseLine::new(1.0, 0.0, 1.0, 0.0, CreaseType::Flat));
        let rotated = cp.apply_fold(&FoldOperation::RotatePoint {
            cx: 0.0,
            cy: 0.0,
            angle: PI,
        });
        assert!((rotated.creases[0].x1 - (-1.0)).abs() < 1e-9);
        assert!(rotated.creases[0].y1.abs() < 1e-9);
    }

    #[test]
    fn test_render_buffer_dimensions() {
        let cp = OrigamiPatterns::preliminary_base();
        let buf = cp.render(64, 64);
        assert_eq!(buf.len(), 64);
        assert_eq!(buf[0].len(), 64);
    }

    #[test]
    fn test_render_draws_pixels() {
        let cp = OrigamiPatterns::preliminary_base();
        let buf = cp.render(128, 128);
        let has_non_white = buf.iter().flatten().any(|&p| p != [255u8; 3]);
        assert!(has_non_white, "render produced a completely white image");
    }

    #[test]
    fn test_add_crease_and_retrieve() {
        let mut cp = CreasePattern::new(2.0, 2.0);
        cp.add_crease(CreaseLine::new(0.0, 0.0, 2.0, 2.0, CreaseType::Mountain));
        assert_eq!(cp.creases.len(), 1);
        assert_eq!(cp.creases[0].crease_type, CreaseType::Mountain);
    }

    #[test]
    fn test_unfold_noop() {
        let cp = OrigamiPatterns::preliminary_base();
        let original_len = cp.creases.len();
        let unfolded = cp.apply_fold(&FoldOperation::Unfold);
        assert_eq!(unfolded.creases.len(), original_len);
    }
}
