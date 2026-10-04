//! CPU-side cost of one wallpaper frame: RK4-step every geodesic, push its
//! position into its trail, and flatten the trails for upload. This is the
//! work the render loop does per frame before the GPU draw.
//!
//! The same file builds against 1.5.1, so the numbers in the README compare
//! releases on one machine. Run with `cargo bench --bench simulation`.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::surface::{
    klein_bottle::KleinBottle, pseudosphere::Pseudosphere, sphere::Sphere, torus::Torus, Surface,
};
use geodesic_wallpaper::trail::TrailBuffer;
use rand::{rngs::StdRng, SeedableRng};
use std::hint::black_box;

const GEODESICS: usize = 30;
const TRAIL: usize = 300;

struct Scene<'a> {
    surf: &'a dyn Surface,
    geos: Vec<Geodesic>,
    trails: Vec<TrailBuffer>,
}

fn scene(surf: &dyn Surface) -> Scene<'_> {
    let mut rng = StdRng::seed_from_u64(42);
    let mut geos = Vec::new();
    let mut trails = Vec::new();
    for i in 0..GEODESICS {
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        geos.push(Geodesic::new(u, v, du, dv, usize::MAX / 2, i));
        trails.push(TrailBuffer::new(TRAIL, [1.0, 1.0, 1.0, 1.0], 2.0));
    }
    let mut s = Scene { surf, geos, trails };
    // Fill the trails so flattening does full work.
    for _ in 0..TRAIL {
        frame(&mut s);
    }
    s
}

fn frame(s: &mut Scene<'_>) -> usize {
    for (g, t) in s.geos.iter_mut().zip(s.trails.iter_mut()) {
        let p = s.surf.position(g.u, g.v);
        t.push([p.x, p.y, p.z]);
        g.step(s.surf, 0.016);
    }
    s.trails.iter().map(|t| t.ordered_vertices().len()).sum()
}

fn bench(c: &mut Criterion) {
    let torus = Torus::new(2.0, 0.7);
    let sphere = Sphere::new(2.0);
    let pseudo = Pseudosphere::new(1.0, 3.0);
    let klein = KleinBottle::new(2.0, 1.0);
    let surfaces: [(&str, &dyn Surface); 4] = [
        ("torus", &torus),
        ("sphere", &sphere),
        ("pseudosphere", &pseudo),
        ("klein_bottle", &klein),
    ];
    let mut g = c.benchmark_group("frame_30_geodesics_300_trail");
    for (name, surf) in surfaces {
        let mut s = scene(surf);
        g.bench_with_input(BenchmarkId::from_parameter(name), &(), |b, _| {
            b.iter(|| black_box(frame(&mut s)))
        });
    }
    g.finish();

    let mut g = c.benchmark_group("rk4_step");
    for (name, surf) in surfaces {
        let mut rng = StdRng::seed_from_u64(7);
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        let mut geo = Geodesic::new(u, v, du, dv, usize::MAX / 2, 0);
        g.bench_with_input(BenchmarkId::from_parameter(name), &(), |b, _| {
            b.iter(|| {
                geo.step(black_box(surf), 0.016);
                black_box(geo.u)
            })
        });
    }
    g.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
