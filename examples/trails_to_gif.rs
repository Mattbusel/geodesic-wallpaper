//! Integrate geodesics on a torus on the CPU, draw their trails into an
//! image buffer and save the animation as a GIF and an APNG. No GPU and no
//! window: this is the library without the wallpaper renderer.
//!
//! Run with `cargo run --example trails_to_gif -- [OUTPUT_DIR]`
//! (default `target/example-output`).

use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::recorder::{ApngWriter, GifWriter};
use geodesic_wallpaper::surface::{torus::Torus, Surface};
use geodesic_wallpaper::trail::TrailBuffer;
use rand::{rngs::StdRng, SeedableRng};

const W: u32 = 320;
const H: u32 = 180;
const FRAMES: u32 = 60;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "target/example-output".into()),
    );
    std::fs::create_dir_all(&out)?;

    let torus = Torus::new(2.0, 0.7);
    let colors = [
        [0.3, 0.6, 1.0, 1.0],
        [1.0, 0.85, 0.2, 1.0],
        [0.5, 1.0, 0.6, 1.0],
    ];
    let mut rng = StdRng::seed_from_u64(7);
    let mut geos = Vec::new();
    let mut trails = Vec::new();
    for i in 0..12 {
        let (u, v) = torus.random_position(&mut rng);
        let (du, dv) = torus.random_tangent(u, v, &mut rng);
        geos.push(Geodesic::new(u, v, du, dv, usize::MAX / 2, i));
        trails.push(TrailBuffer::new(120, colors[i % colors.len()], 2.0));
    }

    let mut gif = GifWriter::create(&out.join("torus.gif"), W, H, 30)?;
    let mut apng = ApngWriter::create(&out.join("torus.png"), W, H, 30, FRAMES)?;
    let mut verts = Vec::new();
    for frame in 0..(FRAMES + 120) {
        for (g, t) in geos.iter_mut().zip(trails.iter_mut()) {
            let p = torus.position(g.u, g.v);
            t.push([p.x, p.y, p.z]);
            g.step(&torus, 0.03);
        }
        if frame < 120 {
            continue; // let the trails grow first
        }
        // Tilted orthographic view, rotating slowly.
        let angle = frame as f32 * 0.02;
        let (s, c) = angle.sin_cos();
        let mut img = vec![0u8; (W * H * 4) as usize];
        for px in img.as_chunks_mut::<4>().0 {
            *px = [5, 5, 16, 255];
        }
        verts.clear();
        for t in &trails {
            t.append_ordered(&mut verts, None);
        }
        for v in &verts {
            let [x, y, z] = v.position;
            let xr = x * c - y * s;
            let yr = x * s + y * c;
            let sx = W as f32 / 2.0 + xr * 48.0;
            let sy = H as f32 / 2.0 + (yr * 0.45 - z) * 48.0;
            if sx < 0.0 || sy < 0.0 || sx >= W as f32 || sy >= H as f32 {
                continue;
            }
            let i = ((sy as u32 * W + sx as u32) * 4) as usize;
            let a = v.color[3];
            for k in 0..3 {
                let add = (v.color[k] * a * 255.0) as u16;
                img[i + k] = (img[i + k] as u16 + add).min(255) as u8;
            }
        }
        gif.push_rgba(&img)?;
        apng.push_rgba(&img)?;
    }
    let n = gif.finish()?;
    apng.finish()?;
    println!(
        "wrote {n} frames to {} and {}",
        out.join("torus.gif").display(),
        out.join("torus.png").display()
    );
    Ok(())
}
