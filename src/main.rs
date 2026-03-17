mod config;
mod surface;
mod geodesic;
mod trail;
mod renderer;
mod wallpaper;

use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use rand::SeedableRng;
use rand::rngs::StdRng;
use windows::Win32::UI::WindowsAndMessaging::{
    PeekMessageW, TranslateMessage, DispatchMessageW, MSG, PM_REMOVE,
};

use config::{Config, SharedConfig};
use surface::{Surface, torus::Torus, sphere::Sphere, saddle::Saddle};
use geodesic::Geodesic;
use trail::TrailBuffer;
use renderer::Renderer;

fn build_surface(cfg: &Config) -> Box<dyn Surface> {
    match cfg.surface.as_str() {
        "sphere" => Box::new(Sphere::new(2.5)),
        "saddle" => Box::new(Saddle::new(2.0)),
        _ => Box::new(Torus::new(cfg.torus_R, cfg.torus_r)),
    }
}

fn parse_colors(cfg: &Config) -> Vec<[f32; 4]> {
    cfg.color_palette.iter().map(|s| Config::parse_color(s)).collect()
}

fn screen_size() -> (i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w > 0 && h > 0 { (w, h) } else { (1920, 1080) }
    }
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Info).init();

    let config_path = PathBuf::from("config.toml");
    let cfg = Config::load(&config_path);
    let shared_cfg: SharedConfig = Arc::new(RwLock::new(cfg.clone()));

    // Hot-reload watcher
    {
        let shared = shared_cfg.clone();
        let path = config_path.clone();
        std::thread::spawn(move || {
            use notify::{Watcher, RecursiveMode, recommended_watcher};
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = recommended_watcher(move |res| { let _ = tx.send(res); }).unwrap();
            let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
            loop {
                if rx.recv().is_ok() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let new_cfg = Config::load(&path);
                    if let Ok(mut w) = shared.write() {
                        *w = new_cfg;
                        log::info!("Config reloaded");
                    }
                }
            }
        });
    }

    let (sw, sh) = screen_size();
    let hwnd = wallpaper::create_wallpaper_hwnd(sw, sh)
        .expect("Failed to create wallpaper window");

    let surf = build_surface(&cfg);
    let colors = parse_colors(&cfg);
    let (mesh_verts, mesh_indices) = surf.mesh_vertices(40, 40);

    let mut renderer = pollster::block_on(Renderer::new(
        hwnd,
        sw as u32, sh as u32,
        &mesh_verts,
        &mesh_indices,
    ));

    let mut rng = StdRng::from_entropy();
    let mut geodesics: Vec<Geodesic> = Vec::new();
    let mut trails: Vec<TrailBuffer> = Vec::new();

    for i in 0..cfg.num_geodesics {
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        let ci = i % colors.len();
        geodesics.push(Geodesic::new(u, v, du, dv, cfg.trail_length, ci));
        trails.push(TrailBuffer::new(cfg.trail_length, colors[ci]));
    }

    // Read dt from config (default 0.016).  The old hardcoded 0.04 caused
    // geodesics on a torus (small_r=0.7) to overshoot the tube circumference
    // in under 4 seconds, producing discontinuous jumps between Christoffel
    // evaluations and visually drifting trajectories.
    let dt = cfg.time_step;
    let target_frame = std::time::Duration::from_millis(33);
    let mut last_frame = std::time::Instant::now();

    // Raw Win32 message loop
    loop {
        // Drain pending messages
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == 0x0012 { // WM_QUIT
                    return;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Frame rate limit
        let now = std::time::Instant::now();
        if now.duration_since(last_frame) < target_frame {
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
        }
        last_frame = std::time::Instant::now();

        // Orbit camera
        let rot = shared_cfg.read().map(|c| c.rotation_speed).unwrap_or(0.001047);
        renderer.camera.orbit(rot * dt);

        // Step geodesics
        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive { continue; }
            let pos = surf.position(geo.u, geo.v);
            trails[i].push([pos.x, pos.y, pos.z]);
            geo.step(surf.as_ref(), dt);
        }

        // Respawn dead
        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive {
                let (u, v) = surf.random_position(&mut rng);
                let (du, dv) = surf.random_tangent(u, v, &mut rng);
                let tl = shared_cfg.read().map(|c| c.trail_length).unwrap_or(300);
                let ci = i % colors.len();
                *geo = Geodesic::new(u, v, du, dv, tl, ci);
                trails[i].clear();
                trails[i].color = colors[ci];
            }
        }

        // Collect trail verts
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
