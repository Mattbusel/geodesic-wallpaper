//! Define a surface from a formula and sample it, the way a `config.toml`
//! user-defined surface does. Bad formulas come back as errors, not panics.
//!
//! Run with `cargo run --example custom_surface`.

use geodesic_wallpaper::surface::user_defined::{UserDefinedSurface, UserSurfaceConfig};

fn main() {
    let cfg = UserSurfaceConfig {
        // A ripple that fades out from the centre. `^` and `**` both mean power.
        z_expr: "sin(sqrt(x^2 + y^2) * 3) * exp(-0.2 * (x**2 + y**2))".into(),
        x_range: [-4.0, 4.0],
        y_range: [-4.0, 4.0],
        z_scale: 1.0,
    };
    let surface = UserDefinedSurface::new(cfg).expect("formula parses");

    println!("z along the x axis:");
    for i in 0..=8 {
        let u = i as f32 / 8.0;
        let [x, y, z] = surface.sample(u, 0.5);
        println!("  x={x:+.1} y={y:+.1}  z={z:+.4}");
    }

    for bad in ["sin(x", "sinn(x)", "x ** ** 2"] {
        let r = UserDefinedSurface::new(UserSurfaceConfig {
            z_expr: bad.into(),
            ..UserSurfaceConfig::default()
        });
        match r {
            Ok(_) => println!("{bad:12} parsed (unexpected)"),
            Err(e) => println!("{bad:12} -> {e}"),
        }
    }
}
