//! Entry point for geodesic-wallpaper.
//!
//! Initialises logging and tracing, loads configuration, spawns the hot-reload
//! watcher, creates the Win32 wallpaper window, builds the wgpu renderer, and
//! runs the main message/render loop.

use geodesic_wallpaper::config::{Config, SharedConfig};
use geodesic_wallpaper::error::GeodesicError;
use geodesic_wallpaper::surface::{Surface, torus::Torus, sphere::Sphere, saddle::Saddle};
use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::trail::TrailBuffer;
use geodesic_wallpaper::renderer::Renderer;
use geodesic_wallpaper::wallpaper;

use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use rand::SeedableRng;
use rand::rngs::StdRng;
use windows::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE,
};

/// Construct the surface implementation selected in `cfg`.
///
/// Unrecognised surface names fall back to the torus.
#[tracing::instrument(skip(cfg), fields(surface = %cfg.surface))]
fn build_surface(cfg: &Config) -> Box<dyn Surface> {
    tracing::info!(surface = %cfg.surface, "building surface");
    match cfg.surface.as_str() {
        "sphere" => Box::new(Sphere::new(2.5)),
        "saddle" => Box::new(Saddle::new(2.0)),
        _ => Box::new(Torus::new(cfg.torus_R, cfg.torus_r)),
    }
}

/// Convert the hex colour palette from config into `[f32; 4]` RGBA values.
fn parse_colors(cfg: &Config) -> Vec<[f32; 4]> {
    cfg.color_palette.iter().map(|s| Config::parse_color(s)).collect()
}

/// Query the primary monitor resolution via Win32.
///
/// Returns `(1920, 1080)` as a safe fallback if the system call fails.
fn screen_size() -> (i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w > 0 && h > 0 { (w, h) } else { (1920, 1080) }
    }
}

fn main() {
    // Initialise tracing-subscriber with env-filter so RUST_LOG controls verbosity.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(e) = run() {
        tracing::error!("Fatal error: {e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Application body returning a typed error on failure.
///
/// Separated from `main` so that `?` propagation can be used throughout
/// and the error message is printed cleanly without a Rust backtrace dump.
#[tracing::instrument]
fn run() -> Result<(), GeodesicError> {
    let config_path = PathBuf::from("config.toml");
    let cfg = Config::load(&config_path);
    tracing::info!(
        surface = %cfg.surface,
        num_geodesics = cfg.num_geodesics,
        trail_length = cfg.trail_length,
        "configuration loaded"
    );
    let shared_cfg: SharedConfig = Arc::new(RwLock::new(cfg.clone()));

    // Spawn hot-reload watcher thread.
    {
        let shared = shared_cfg.clone();
        let path = config_path.clone();
        std::thread::spawn(move || {
            use notify::{Watcher, RecursiveMode, recommended_watcher};
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match recommended_watcher(move |res| { let _ = tx.send(res); }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("Failed to start config watcher: {e}");
                    return;
                }
            };
            let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
            loop {
                if rx.recv().is_ok() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let new_cfg = Config::load(&path);
                    if let Ok(mut w) = shared.write() {
                        *w = new_cfg;
                        tracing::info!("config reloaded from disk");
                    }
                }
            }
        });
    }

    let (sw, sh) = screen_size();
    tracing::info!(width = sw, height = sh, "detected screen resolution");
    let hwnd = wallpaper::create_wallpaper_hwnd(sw, sh)
        .ok_or_else(|| GeodesicError::window("Failed to create wallpaper window"))?;

    let surf = build_surface(&cfg);
    let colors = parse_colors(&cfg);
    let (mesh_verts, mesh_indices) = surf.mesh_vertices(40, 40);
    tracing::info!(
        verts = mesh_verts.len(),
        indices = mesh_indices.len(),
        "surface mesh generated"
    );

    let mut renderer = pollster::block_on(Renderer::new(
        hwnd,
        sw as u32, sh as u32,
        &mesh_verts,
        &mesh_indices,
    ))?;

    let mut rng = StdRng::from_entropy();
    let mut geodesics: Vec<Geodesic> = Vec::new();
    let mut trails: Vec<TrailBuffer> = Vec::new();

    for i in 0..cfg.num_geodesics {
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        let ci = i % colors.len().max(1);
        geodesics.push(Geodesic::new(u, v, du, dv, cfg.trail_length, ci));
        trails.push(TrailBuffer::new(cfg.trail_length, colors[ci]));
    }
    tracing::info!(count = cfg.num_geodesics, "geodesics spawned");

    let dt = cfg.time_step;
    let target_frame = std::time::Duration::from_millis(33);
    let mut last_frame = std::time::Instant::now();

    tracing::info!("entering render loop");

    // Raw Win32 message loop.
    loop {
        // Drain pending messages.
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == 0x0012 { // WM_QUIT
                    tracing::info!("WM_QUIT received, exiting");
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Frame rate limit.
        let now = std::time::Instant::now();
        if now.duration_since(last_frame) < target_frame {
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }
        last_frame = std::time::Instant::now();

        // Orbit camera.
        let rot = shared_cfg.read().map(|c| c.rotation_speed).unwrap_or(0.001047);
        renderer.camera.orbit(rot * dt);

        // Step geodesics.
        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive { continue; }
            let pos = surf.position(geo.u, geo.v);
            trails[i].push([pos.x, pos.y, pos.z]);
            geo.step(surf.as_ref(), dt);
        }

        // Respawn dead geodesics.
        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive {
                let (u, v) = surf.random_position(&mut rng);
                let (du, dv) = surf.random_tangent(u, v, &mut rng);
                let tl = shared_cfg.read().map(|c| c.trail_length).unwrap_or(300);
                let ci = i % colors.len().max(1);
                *geo = Geodesic::new(u, v, du, dv, tl, ci);
                trails[i].clear();
                trails[i].color = colors[ci];
            }
        }

        // Collect trail vertices.
        let mut all_verts = Vec::new();
        let mut seg_lens = Vec::new();
        for trail in &trails {
            let v = trail.ordered_vertices();
            seg_lens.push(v.len());
            all_verts.extend(v);
        }

        renderer.render(&all_verts, &seg_lens);
    }
}
