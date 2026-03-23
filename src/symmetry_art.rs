//! Wallpaper group patterns and frieze patterns.

/// Wallpaper group enumeration (17 plane symmetry groups).
#[derive(Debug, Clone, PartialEq)]
pub enum WallpaperGroup {
    P1,
    P2,
    PM,
    PG,
    CM,
    PMM,
    PMG,
    PGG,
    CMM,
    P4,
    P4M,
    P4G,
    P3,
    P3M1,
    P31M,
    P6,
    P6M,
}

/// Frieze group enumeration (7 frieze symmetry groups).
#[derive(Debug, Clone, PartialEq)]
pub enum FriezeGroup {
    F1,
    F2,
    F11,
    F1M,
    FM,
    F2MM,
    F2MG,
}

/// A 2D symmetry transform.
#[derive(Debug, Clone)]
pub struct SymmetryTransform {
    pub translate: (f64, f64),
    pub rotate_deg: f64,
    pub reflect_x: bool,
    pub reflect_y: bool,
}

impl SymmetryTransform {
    pub fn identity() -> Self {
        Self {
            translate: (0.0, 0.0),
            rotate_deg: 0.0,
            reflect_x: false,
            reflect_y: false,
        }
    }
}

/// Apply a transform to a point.
pub fn apply_transform(x: f64, y: f64, t: &SymmetryTransform) -> (f64, f64) {
    let mut px = x;
    let mut py = y;

    // Reflect
    if t.reflect_x {
        py = -py;
    }
    if t.reflect_y {
        px = -px;
    }

    // Rotate
    if t.rotate_deg.abs() > 1e-9 {
        let angle = t.rotate_deg.to_radians();
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        let rx = cos_a * px - sin_a * py;
        let ry = sin_a * px + cos_a * py;
        px = rx;
        py = ry;
    }

    // Translate
    px += t.translate.0;
    py += t.translate.1;

    (px, py)
}

fn make_rotation(deg: f64) -> SymmetryTransform {
    SymmetryTransform {
        translate: (0.0, 0.0),
        rotate_deg: deg,
        reflect_x: false,
        reflect_y: false,
    }
}

fn make_translation(tx: f64, ty: f64) -> SymmetryTransform {
    SymmetryTransform {
        translate: (tx, ty),
        rotate_deg: 0.0,
        reflect_x: false,
        reflect_y: false,
    }
}

fn make_reflect_x_translate(tx: f64, ty: f64) -> SymmetryTransform {
    SymmetryTransform {
        translate: (tx, ty),
        rotate_deg: 0.0,
        reflect_x: true,
        reflect_y: false,
    }
}

fn make_reflect_y_translate(tx: f64, ty: f64) -> SymmetryTransform {
    SymmetryTransform {
        translate: (tx, ty),
        rotate_deg: 0.0,
        reflect_x: false,
        reflect_y: true,
    }
}

/// Return fundamental domain transforms for a wallpaper group.
pub fn wallpaper_transforms(
    group: &WallpaperGroup,
    lattice_a: f64,
    lattice_b: f64,
) -> Vec<SymmetryTransform> {
    match group {
        WallpaperGroup::P1 => vec![SymmetryTransform::identity()],
        WallpaperGroup::P2 => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
        ],
        WallpaperGroup::PM => vec![
            SymmetryTransform::identity(),
            make_reflect_y_translate(0.0, 0.0),
            make_translation(lattice_a, 0.0),
            SymmetryTransform {
                translate: (lattice_a, 0.0),
                rotate_deg: 0.0,
                reflect_x: false,
                reflect_y: true,
            },
        ],
        WallpaperGroup::PG => vec![
            SymmetryTransform::identity(),
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: false,
            },
        ],
        WallpaperGroup::CM => vec![
            SymmetryTransform::identity(),
            make_reflect_x_translate(0.0, 0.0),
            make_translation(lattice_a / 2.0, lattice_b / 2.0),
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: false,
            },
        ],
        WallpaperGroup::PMM => vec![
            SymmetryTransform::identity(),
            make_reflect_x_translate(0.0, 0.0),
            make_reflect_y_translate(0.0, 0.0),
            SymmetryTransform {
                translate: (0.0, 0.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: true,
            },
        ],
        WallpaperGroup::PMG => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
            make_reflect_x_translate(lattice_a / 2.0, 0.0),
            SymmetryTransform {
                translate: (lattice_a / 2.0, 0.0),
                rotate_deg: 180.0,
                reflect_x: true,
                reflect_y: false,
            },
        ],
        WallpaperGroup::PGG => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: false,
            },
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 0.0,
                reflect_x: false,
                reflect_y: true,
            },
        ],
        WallpaperGroup::CMM => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
            make_reflect_x_translate(0.0, 0.0),
            make_reflect_y_translate(0.0, 0.0),
            make_translation(lattice_a / 2.0, lattice_b / 2.0),
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 180.0,
                reflect_x: false,
                reflect_y: false,
            },
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: false,
            },
            SymmetryTransform {
                translate: (lattice_a / 2.0, lattice_b / 2.0),
                rotate_deg: 0.0,
                reflect_x: false,
                reflect_y: true,
            },
        ],
        WallpaperGroup::P4 => vec![
            SymmetryTransform::identity(),
            make_rotation(90.0),
            make_rotation(180.0),
            make_rotation(270.0),
        ],
        WallpaperGroup::P4M => {
            let mut t = vec![
                SymmetryTransform::identity(),
                make_rotation(90.0),
                make_rotation(180.0),
                make_rotation(270.0),
            ];
            for deg in [0.0, 90.0, 180.0, 270.0] {
                t.push(SymmetryTransform {
                    translate: (0.0, 0.0),
                    rotate_deg: deg,
                    reflect_x: true,
                    reflect_y: false,
                });
            }
            t
        }
        WallpaperGroup::P4G => {
            let mut t = vec![
                SymmetryTransform::identity(),
                make_rotation(90.0),
                make_rotation(180.0),
                make_rotation(270.0),
            ];
            let off = lattice_a / 2.0;
            for deg in [0.0, 90.0, 180.0, 270.0] {
                t.push(SymmetryTransform {
                    translate: (off, off),
                    rotate_deg: deg,
                    reflect_x: true,
                    reflect_y: false,
                });
            }
            t
        }
        WallpaperGroup::P3 => {
            let deg = 120.0_f64;
            vec![
                SymmetryTransform::identity(),
                make_rotation(deg),
                make_rotation(2.0 * deg),
            ]
        }
        WallpaperGroup::P3M1 => {
            let deg = 120.0_f64;
            let mut t = vec![
                SymmetryTransform::identity(),
                make_rotation(deg),
                make_rotation(2.0 * deg),
            ];
            for d in [0.0, deg, 2.0 * deg] {
                t.push(SymmetryTransform {
                    translate: (0.0, 0.0),
                    rotate_deg: d,
                    reflect_x: true,
                    reflect_y: false,
                });
            }
            t
        }
        WallpaperGroup::P31M => {
            let deg = 120.0_f64;
            let mut t = vec![
                SymmetryTransform::identity(),
                make_rotation(deg),
                make_rotation(2.0 * deg),
            ];
            for d in [0.0, deg, 2.0 * deg] {
                t.push(SymmetryTransform {
                    translate: (0.0, 0.0),
                    rotate_deg: d,
                    reflect_x: false,
                    reflect_y: true,
                });
            }
            t
        }
        WallpaperGroup::P6 => (0..6)
            .map(|i| make_rotation(i as f64 * 60.0))
            .collect(),
        WallpaperGroup::P6M => {
            let mut t: Vec<SymmetryTransform> =
                (0..6).map(|i| make_rotation(i as f64 * 60.0)).collect();
            for i in 0..6 {
                t.push(SymmetryTransform {
                    translate: (0.0, 0.0),
                    rotate_deg: i as f64 * 60.0,
                    reflect_x: true,
                    reflect_y: false,
                });
            }
            t
        }
    }
}

/// Return transforms for a frieze group along the X axis.
pub fn frieze_transforms(group: &FriezeGroup, repeat: f64) -> Vec<SymmetryTransform> {
    match group {
        FriezeGroup::F1 => vec![SymmetryTransform::identity()],
        FriezeGroup::F2 => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
        ],
        FriezeGroup::F11 => vec![
            SymmetryTransform::identity(),
            // Glide reflection: reflect y, translate x by half
            SymmetryTransform {
                translate: (repeat / 2.0, 0.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: false,
            },
        ],
        FriezeGroup::F1M => vec![
            SymmetryTransform::identity(),
            // Vertical mirror
            make_reflect_y_translate(0.0, 0.0),
        ],
        FriezeGroup::FM => vec![
            SymmetryTransform::identity(),
            // Horizontal mirror
            make_reflect_x_translate(0.0, 0.0),
        ],
        FriezeGroup::F2MM => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
            make_reflect_x_translate(0.0, 0.0),
            make_reflect_y_translate(0.0, 0.0),
        ],
        FriezeGroup::F2MG => vec![
            SymmetryTransform::identity(),
            make_rotation(180.0),
            SymmetryTransform {
                translate: (repeat / 2.0, 0.0),
                rotate_deg: 0.0,
                reflect_x: true,
                reflect_y: false,
            },
            SymmetryTransform {
                translate: (repeat / 2.0, 0.0),
                rotate_deg: 180.0,
                reflect_x: true,
                reflect_y: false,
            },
        ],
    }
}

/// Apply all transforms to each point in the motif.
pub fn tile_motif(
    motif_points: &[(f64, f64)],
    transforms: &[SymmetryTransform],
) -> Vec<(f64, f64)> {
    let mut result = Vec::with_capacity(motif_points.len() * transforms.len());
    for &(x, y) in motif_points {
        for t in transforms {
            result.push(apply_transform(x, y, t));
        }
    }
    result
}

/// Draw a filled circle onto an RGB image buffer.
pub fn draw_circle(image: &mut Vec<u8>, width: u32, cx: f64, cy: f64, r: f64, color: [u8; 3]) {
    let height = image.len() as u32 / (width * 3);
    let x_min = ((cx - r).floor() as i64).max(0) as u32;
    let x_max = ((cx + r).ceil() as i64).min(width as i64 - 1) as u32;
    let y_min = ((cy - r).floor() as i64).max(0) as u32;
    let y_max = ((cy + r).ceil() as i64).min(height as i64 - 1) as u32;

    for py in y_min..=y_max {
        for px in x_min..=x_max {
            let dx = px as f64 - cx;
            let dy = py as f64 - cy;
            if dx * dx + dy * dy <= r * r {
                let idx = ((py * width + px) * 3) as usize;
                if idx + 2 < image.len() {
                    image[idx] = color[0];
                    image[idx + 1] = color[1];
                    image[idx + 2] = color[2];
                }
            }
        }
    }
}

/// LCG random number generator returning f64 in [0, 1).
fn lcg_next(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    (*state >> 33) as f64 / u32::MAX as f64
}

/// Render a wallpaper pattern as an RGB image.
pub fn render_wallpaper(group: &WallpaperGroup, width: u32, height: u32, seed: u64) -> Vec<u8> {
    let mut image = vec![0u8; (width * height * 3) as usize];
    let transforms = wallpaper_transforms(group, 1.0, 1.0);

    // Generate random motif: 5-10 points in [0,1]²
    let mut state = seed.wrapping_add(1);
    let num_pts = 5 + (lcg_next(&mut state) * 6.0) as usize;
    let motif: Vec<(f64, f64)> = (0..num_pts)
        .map(|_| (lcg_next(&mut state), lcg_next(&mut state)))
        .collect();

    let tiled = tile_motif(&motif, &transforms);

    let r_px = (width.min(height) as f64 * 0.02).max(2.0);

    // Tile across the image using a grid of repeating cells
    let tile_w = width as f64;
    let tile_h = height as f64;
    for &(fx, fy) in &tiled {
        // Map normalized coords to pixel coords, tile with modulo
        let px = ((fx % 1.0 + 1.0) % 1.0 * tile_w) as f64;
        let py = ((fy % 1.0 + 1.0) % 1.0 * tile_h) as f64;
        draw_circle(&mut image, width, px, py, r_px, [255, 255, 255]);
    }

    image
}

/// Render a frieze pattern as an RGB image.
pub fn render_frieze(group: &FriezeGroup, width: u32, height: u32) -> Vec<u8> {
    let mut image = vec![0u8; (width * height * 3) as usize];
    let transforms = frieze_transforms(group, 1.0);

    // Simple motif: a triangle of 3 points
    let motif: Vec<(f64, f64)> = vec![(0.1, 0.3), (0.15, 0.7), (0.05, 0.7)];
    let tiled = tile_motif(&motif, &transforms);

    let r_px = (width.min(height) as f64 * 0.04).max(2.0);

    for &(fx, fy) in &tiled {
        let px = ((fx % 1.0 + 1.0) % 1.0 * width as f64) as f64;
        let py = ((fy % 1.0 + 1.0) % 1.0 * height as f64) as f64;
        draw_circle(&mut image, width, px, py, r_px, [255, 255, 255]);
    }

    image
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_transform_identity() {
        let t = SymmetryTransform::identity();
        let (x, y) = apply_transform(3.0, 4.0, &t);
        assert!((x - 3.0).abs() < 1e-9);
        assert!((y - 4.0).abs() < 1e-9);
    }

    #[test]
    fn test_p4_produces_4_transforms() {
        let transforms = wallpaper_transforms(&WallpaperGroup::P4, 1.0, 1.0);
        assert_eq!(transforms.len(), 4);
    }

    #[test]
    fn test_frieze_f2mm_produces_expected_count() {
        let transforms = frieze_transforms(&FriezeGroup::F2MM, 1.0);
        assert_eq!(transforms.len(), 4);
    }

    #[test]
    fn test_tile_motif_multiplies_points() {
        let motif = vec![(0.5, 0.5), (0.1, 0.2)];
        let transforms = wallpaper_transforms(&WallpaperGroup::P4, 1.0, 1.0);
        let tiled = tile_motif(&motif, &transforms);
        assert_eq!(tiled.len(), motif.len() * transforms.len());
    }

    #[test]
    fn test_render_returns_correct_buffer_size() {
        let img = render_wallpaper(&WallpaperGroup::P4, 64, 64, 42);
        assert_eq!(img.len(), 64 * 64 * 3);
        let frieze_img = render_frieze(&FriezeGroup::F2MM, 128, 32);
        assert_eq!(frieze_img.len(), 128 * 32 * 3);
    }
}
