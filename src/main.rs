//! Entry point for geodesic-wallpaper.
//!
//! Initialises logging and tracing, loads configuration, spawns the hot-reload
//! watcher, creates the Win32 wallpaper window, builds the wgpu renderer, and
//! runs the main message/render loop.

use clap::Parser;
use geodesic_wallpaper::config::{Config, SharedConfig};
use geodesic_wallpaper::error::GeodesicError;
use geodesic_wallpaper::events::KeyEvent;
use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::renderer::Renderer;
use geodesic_wallpaper::surface::{
    catenoid::Catenoid, enneper::Enneper, helicoid::Helicoid, hyperboloid::Hyperboloid,
    saddle::Saddle, sphere::Sphere, torus::Torus, Surface,
};
use geodesic_wallpaper::trail::TrailBuffer;
use geodesic_wallpaper::tray;
use geodesic_wallpaper::wallpaper;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::prelude::*;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MessageBoxW, PeekMessageW, TranslateMessage, MSG, MB_ICONWARNING, MB_OK,
    PM_REMOVE,
};

/// Command-line arguments.
#[derive(Parser)]
#[command(about = "Geodesic wallpaper")]
struct Args {
    /// Run headless, render N frames and save screenshot.
    #[arg(long)]
    headless: bool,
    /// Output file path for headless screenshot.
    #[arg(long, default_value = "screenshot.png")]
    output: String,
    /// Number of frames to simulate in headless mode.
    #[arg(long, default_value_t = 300)]
    frames: u32,
}

/// Surface names in cycle order.
const SURFACE_CYCLE: &[&str] = &[
    "torus",
    "sphere",
    "saddle",
    "catenoid",
    "helicoid",
    "hyperboloid",
];

/// Construct the surface implementation selected in `cfg`.
///
/// Unrecognised surface names fall back to the torus.
#[tracing::instrument(skip(cfg), fields(surface = %cfg.surface))]
fn build_surface(cfg: &Config) -> Arc<dyn Surface> {
    tracing::info!(surface = %cfg.surface, "building surface");
    match cfg.surface.as_str() {
        "sphere" => Arc::new(Sphere::new(2.5)),
        "saddle" => Arc::new(Saddle::new(2.0)),
        "enneper" => Arc::new(Enneper::new(1.5)),
        "catenoid" => Arc::new(Catenoid::new(cfg.catenoid_c)),
        "helicoid" => Arc::new(Helicoid::new(cfg.helicoid_c)),
        "hyperboloid" => Arc::new(Hyperboloid::new(cfg.hyperboloid_a, cfg.hyperboloid_b)),
        _ => Arc::new(Torus::new(cfg.torus_R, cfg.torus_r)),
    }
}

/// Build a surface by name, ignoring config parameters (used for cycle).
fn build_surface_by_name(name: &str) -> Arc<dyn Surface> {
    match name {
        "sphere" => Arc::new(Sphere::new(2.5)),
        "saddle" => Arc::new(Saddle::new(2.0)),
        "enneper" => Arc::new(Enneper::new(1.5)),
        "catenoid" => Arc::new(Catenoid::new(1.0)),
        "helicoid" => Arc::new(Helicoid::new(1.0)),
        "hyperboloid" => Arc::new(Hyperboloid::new(1.0, 1.0)),
        _ => Arc::new(Torus::new(2.0, 0.7)),
    }
}

/// Convert the hex colour palette from config into `[f32; 4]` RGBA values.
fn parse_colors(cfg: &Config) -> Vec<[f32; 4]> {
    cfg.effective_colors()
}

/// Rotate hue of a set of colours by `degrees` (0–360).
fn rotate_hue(colors: &[[f32; 4]], degrees: f32) -> Vec<[f32; 4]> {
    let shift = degrees / 360.0;
    colors
        .iter()
        .map(|&c| {
            let hsv = rgb_to_hsv(c);
            let h = (hsv[0] + shift).rem_euclid(1.0);
            hsv_to_rgb([h, hsv[1], hsv[2]])
        })
        .collect()
}

/// Pick a color index for geodesic `i` given the color mode.
fn pick_color_idx(i: usize, color_count: usize, mode: &str, rng: &mut StdRng) -> usize {
    if mode == "random" {
        rng.gen_range(0..color_count.max(1))
    } else {
        i % color_count.max(1)
    }
}

fn rgb_to_hsv(c: [f32; 4]) -> [f32; 3] {
    let (r, g, b) = (c[0], c[1], c[2]);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max < 1e-6 { 0.0 } else { delta / max };
    let h = if delta < 1e-6 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    [h, s, v]
}

fn hsv_to_rgb(hsv: [f32; 3]) -> [f32; 4] {
    let (h, s, v) = (hsv[0], hsv[1], hsv[2]);
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r, g, b, 1.0]
}

/// Parse a CSS hex colour string to `(r, g, b)` in `f64` linear range 0–1.
fn parse_bg_color(hex: &str) -> (f64, f64, f64) {
    let c = Config::parse_color(hex);
    (c[0] as f64, c[1] as f64, c[2] as f64)
}

/// Query the primary monitor resolution via Win32.
///
/// Returns `(1920, 1080)` as a safe fallback if the system call fails.
fn screen_size(multi_monitor: bool) -> (i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        };
        if multi_monitor {
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            if w > 0 && h > 0 {
                return (w, h);
            }
        }
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w > 0 && h > 0 { (w, h) } else { (1920, 1080) }
    }
}

/// Show an epilepsy/photosensitivity warning using a Win32 message box.
fn show_epilepsy_warning() {
    unsafe {
        let text: Vec<u16> = "Geodesic Wallpaper displays animated moving patterns.\n\n\
            If you or anyone in your household is photosensitive or has a history \
            of epilepsy, please be aware that the animated visuals may trigger \
            symptoms.\n\nPress OK to continue."
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();
        let caption: Vec<u16> = "Photosensitivity Notice"
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();
        let _ = MessageBoxW(
            None,
            windows::core::PCWSTR(text.as_ptr()),
            windows::core::PCWSTR(caption.as_ptr()),
            MB_ICONWARNING | MB_OK,
        );
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    if args.headless {
        let config_path = PathBuf::from("config.toml");
        let cfg = Config::load(&config_path).resolve_profile();
        if let Err(e) = run_headless(&args, &cfg) {
            tracing::error!("Headless render failed: {e}");
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
        return;
    }

    if let Err(e) = run() {
        tracing::error!("Fatal error: {e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Headless render path: simulate N frames and save the last frame as a PNG.
fn run_headless(args: &Args, cfg: &Config) -> Result<(), GeodesicError> {
    tracing::info!(frames = args.frames, output = %args.output, "starting headless render");

    let surf = build_surface(cfg);
    let (mesh_verts, mesh_indices) = surf.mesh_vertices(40, 40);

    let width = 1920u32;
    let height = 1080u32;

    let (mut renderer, offscreen_tex) = pollster::block_on(Renderer::new_headless(
        width,
        height,
        &mesh_verts,
        &mesh_indices,
    ))?;

    // Apply config settings.
    {
        let (br, bg, bb) = parse_bg_color(&cfg.background_color);
        renderer.set_background(br, bg, bb);
    }
    renderer.light_dir = cfg.light_dir;

    let mut rng = match cfg.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::seed_from_u64(42),
    };

    let colors = parse_colors(cfg);
    let mut geodesics: Vec<Geodesic> = Vec::new();
    let mut trails: Vec<TrailBuffer> = Vec::new();
    for i in 0..cfg.num_geodesics {
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        let ci = i % colors.len().max(1);
        geodesics.push(Geodesic::new(u, v, du, dv, cfg.trail_length, ci));
        trails.push(TrailBuffer::new(cfg.trail_length, colors[ci], cfg.effective_fade_power()));
    }

    let dt = cfg.time_step;

    // Simulate frames.
    for _frame in 0..args.frames {
        renderer.camera.orbit(cfg.rotation_speed * dt);

        let surf_ref = &*surf;
        geodesics
            .iter_mut()
            .zip(trails.iter_mut())
            .for_each(|(geo, trail)| {
                if !geo.alive { return; }
                let pos = surf_ref.position(geo.u, geo.v);
                trail.push([pos.x, pos.y, pos.z]);
                geo.step(surf_ref, dt);
            });

        // Respawn dead geodesics.
        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive {
                let (u, v) = surf.random_position(&mut rng);
                let (du, dv) = surf.random_tangent(u, v, &mut rng);
                let ci = i % colors.len().max(1);
                *geo = Geodesic::new(u, v, du, dv, cfg.trail_length, ci);
                trails[i].clear();
                trails[i].color = colors[ci];
            }
        }
    }

    // Collect trail vertices for the final frame render.
    let mut all_verts = Vec::new();
    let mut seg_lens = Vec::new();
    for (trail_idx, trail) in trails.iter().enumerate() {
        let ci = geodesics[trail_idx].color_idx % colors.len().max(1);
        let [dr, dg, db, _] = colors[ci];
        let mut v = trail.ordered_vertices();
        for vert in &mut v {
            vert.color[0] = dr;
            vert.color[1] = dg;
            vert.color[2] = db;
        }
        seg_lens.push(v.len());
        all_verts.extend(v);
    }

    // Render to offscreen texture and read back pixels.
    let pixels = renderer.render_to_texture(&offscreen_tex, &all_verts, &seg_lens)?;

    // Save as PNG.
    image::save_buffer(
        &args.output,
        &pixels,
        width,
        height,
        image::ColorType::Rgba8,
    )
    .map_err(|e| GeodesicError::render(format!("image save failed: {e}")))?;

    tracing::info!(path = %args.output, "screenshot saved");
    Ok(())
}

/// Application body returning a typed error on failure.
#[tracing::instrument]
fn run() -> Result<(), GeodesicError> {
    let config_path = PathBuf::from("config.toml");
    let cfg = Config::load(&config_path).resolve_profile();
    tracing::info!(
        surface = %cfg.surface,
        num_geodesics = cfg.num_geodesics,
        trail_length = cfg.trail_length,
        target_fps = cfg.target_fps,
        multi_monitor = cfg.multi_monitor,
        "configuration loaded"
    );

    // Epilepsy warning on startup (can be disabled in config).
    if cfg.epilepsy_warning {
        show_epilepsy_warning();
    }

    let shared_cfg: SharedConfig = Arc::new(RwLock::new(cfg.clone()));

    // Channel for config-reload notifications (used for flash effect).
    let (reload_tx, reload_rx) = std::sync::mpsc::channel::<()>();

    // Spawn hot-reload watcher thread.
    {
        let shared = shared_cfg.clone();
        let path = config_path.clone();
        std::thread::spawn(move || {
            use notify::{recommended_watcher, RecursiveMode, Watcher};
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match recommended_watcher(move |res| {
                let _ = tx.send(res);
            }) {
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
                    let new_cfg = Config::load(&path).resolve_profile();
                    if let Ok(mut w) = shared.write() {
                        *w = new_cfg;
                        tracing::info!("config reloaded from disk");
                    }
                    let _ = reload_tx.send(());
                }
            }
        });
    }

    // Spawn system tray icon.
    let tray_state = tray::spawn_tray(cfg.surface.clone());

    // Set up key event channel.
    let (key_tx, key_rx) = std::sync::mpsc::channel::<KeyEvent>();
    wallpaper::set_key_sender(key_tx);

    // Log monitor enumeration for multi-monitor awareness.
    let monitors = wallpaper::enumerate_monitors();
    tracing::info!(
        monitor_count = monitors.len(),
        "detected monitors (multi-monitor rendering is a future feature)"
    );
    for (i, (x, y, w, h)) in monitors.iter().enumerate() {
        tracing::info!(idx = i, x, y, width = w, height = h, "monitor");
    }

    let (sw, sh) = screen_size(cfg.multi_monitor);
    tracing::info!(width = sw, height = sh, "detected screen resolution");
    let hwnd = wallpaper::create_wallpaper_hwnd(sw, sh)
        .ok_or_else(|| GeodesicError::window("Failed to create wallpaper window"))?;

    let mut surf = build_surface(&cfg);
    let mut colors = parse_colors(&cfg);
    let fade_power = cfg.effective_fade_power();
    let (mesh_verts, mesh_indices) = surf.mesh_vertices(40, 40);
    tracing::info!(
        verts = mesh_verts.len(),
        indices = mesh_indices.len(),
        "surface mesh generated"
    );

    let mut renderer = pollster::block_on(Renderer::new(
        hwnd,
        sw as u32,
        sh as u32,
        &mesh_verts,
        &mesh_indices,
        cfg.max_trail_verts,
        cfg.show_wireframe,
    ))?;

    // Set camera from config.
    renderer.camera = geodesic_wallpaper::renderer::camera::Camera::new_with_params(
        sw as f32 / sh as f32,
        cfg.camera_distance,
        cfg.camera_elevation,
        cfg.camera_fov,
        cfg.camera_elevation_speed,
    );

    // Apply initial background color from config.
    {
        let (br, bg, bb) = parse_bg_color(&cfg.background_color);
        renderer.set_background(br, bg, bb);
    }

    // Set initial light direction.
    renderer.light_dir = cfg.light_dir;

    // Seeded RNG.
    let mut rng = match cfg.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    let mut geodesics: Vec<Geodesic> = Vec::new();
    let mut trails: Vec<TrailBuffer> = Vec::new();

    let color_mode = cfg.color_mode.clone();
    for i in 0..cfg.num_geodesics {
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        let ci = pick_color_idx(i, colors.len(), &color_mode, &mut rng);
        geodesics.push(Geodesic::new(u, v, du, dv, cfg.trail_length, ci));
        trails.push(TrailBuffer::new(cfg.trail_length, colors[ci], fade_power));
    }
    tracing::info!(count = cfg.num_geodesics, "geodesics spawned");

    let dt = cfg.time_step;
    let mut target_frame: Duration;
    let mut last_frame = std::time::Instant::now();

    // HUD tracking.
    let mut hud_last = std::time::Instant::now();
    let mut frame_count: u64 = 0;

    // Current surface name (may be changed by tray).
    let mut current_surface_name = cfg.surface.clone();

    // Timing and animation state.
    let mut elapsed_secs: f32 = 0.0;
    let mut reload_flash_timer: f32 = 0.0;

    // FPS tracking.
    let mut frame_times: VecDeque<Duration> = VecDeque::with_capacity(60);
    let mut fps_log_timer: f32 = 0.0;

    // Surface cycle index.
    let mut surface_idx: usize = SURFACE_CYCLE
        .iter()
        .position(|&s| s == cfg.surface.as_str())
        .unwrap_or(0);

    // Preset cycling state.
    let mut preset_timer: f32 = 0.0;
    let mut preset_idx: usize = 0;

    tracing::info!("entering render loop");

    loop {
        // Check for tray-requested quit.
        if tray_state.quit_requested() {
            tracing::info!("quit requested via tray icon");
            return Ok(());
        }

        // Check for tray surface switch request.
        if let Some(new_surface) = tray_state.take_surface_request() {
            if new_surface != current_surface_name {
                tracing::info!(surface = %new_surface, "switching surface via tray");
                current_surface_name = new_surface.clone();
                // Update shared config surface field.
                if let Ok(mut w) = shared_cfg.write() {
                    w.surface = new_surface.clone();
                }
                surf = build_surface_by_name(&new_surface);
                let (mv, mi) = surf.mesh_vertices(40, 40);
                // Rebuild renderer with new mesh (requires new GPU buffers).
                let show_wire = renderer.show_wireframe;
                let max_tv = renderer.trail_vbuf_capacity();
                renderer = pollster::block_on(Renderer::new(
                    hwnd,
                    sw as u32,
                    sh as u32,
                    &mv,
                    &mi,
                    max_tv,
                    show_wire,
                ))?;
                renderer.camera = geodesic_wallpaper::renderer::camera::Camera::new_with_params(
                    sw as f32 / sh as f32,
                    cfg.camera_distance,
                    cfg.camera_elevation,
                    cfg.camera_fov,
                    cfg.camera_elevation_speed,
                );
                // Respawn all geodesics on new surface.
                geodesics.clear();
                trails.clear();
                let (tl, ng, fp, cm) = {
                    let c = shared_cfg.read().map(|c| (c.trail_length, c.num_geodesics, c.effective_fade_power(), c.color_mode.clone())).unwrap_or((300, 30, 2.0, "cycle".into()));
                    c
                };
                for i in 0..ng {
                    let (u, v) = surf.random_position(&mut rng);
                    let (du, dv) = surf.random_tangent(u, v, &mut rng);
                    let ci = pick_color_idx(i, colors.len(), &cm, &mut rng);
                    geodesics.push(Geodesic::new(u, v, du, dv, tl, ci));
                    trails.push(TrailBuffer::new(tl, colors[ci], fp));
                }
            }
        }

        // Drain pending Win32 messages.
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == 0x0012 {
                    // WM_QUIT
                    tracing::info!("WM_QUIT received, exiting");
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Adaptive frame rate limit.
        {
            let fps = shared_cfg
                .read()
                .map(|c| c.effective_target_fps())
                .unwrap_or(30);
            target_frame = Duration::from_micros(1_000_000 / fps as u64);
        }
        let now = std::time::Instant::now();
        let frame_dt = now.duration_since(last_frame);
        if frame_dt < target_frame {
            let remaining = target_frame - frame_dt;
            if remaining > Duration::from_millis(1) {
                std::thread::sleep(remaining - Duration::from_micros(500));
            }
            continue;
        }
        let frame_dt_secs = frame_dt.as_secs_f32().min(0.1); // cap to avoid spiral on lag
        last_frame = std::time::Instant::now();
        frame_count += 1;

        // Track elapsed time.
        elapsed_secs += frame_dt_secs;
        renderer.elapsed_secs = elapsed_secs;

        // HUD: log FPS every second if enabled.
        {
            let show_hud = shared_cfg.read().map(|c| c.show_hud).unwrap_or(false);
            if show_hud {
                let hud_elapsed = hud_last.elapsed();
                if hud_elapsed >= Duration::from_secs(1) {
                    let fps = frame_count as f64 / hud_elapsed.as_secs_f64();
                    tracing::info!(
                        fps = format!("{fps:.1}"),
                        surface = %current_surface_name,
                        geodesics = geodesics.len(),
                        "HUD"
                    );
                    hud_last = std::time::Instant::now();
                    frame_count = 0;
                }
            }
        }

        // Skip stepping/rendering while paused.
        if tray_state.is_paused() {
            continue;
        }

        // Sync renderer wireframe toggle from config.
        {
            let show_wire = shared_cfg.read().map(|c| c.show_wireframe).unwrap_or(true);
            renderer.show_wireframe = show_wire;
        }

        // Track FPS.
        frame_times.push_back(frame_dt);
        if frame_times.len() > 60 {
            frame_times.pop_front();
        }
        fps_log_timer += frame_dt_secs;
        if fps_log_timer >= 5.0 {
            fps_log_timer = 0.0;
            if !frame_times.is_empty() {
                let avg_ms: f32 = frame_times.iter().map(|d| d.as_secs_f32() * 1000.0).sum::<f32>()
                    / frame_times.len() as f32;
                let fps = 1000.0 / avg_ms;
                if renderer.show_fps_hud {
                    tracing::info!(fps = fps as u32, frame_ms = avg_ms as u32, "FPS");
                }
            }
        }

        // Preset cycling: advance active_profile on a timer.
        {
            let (cycle_secs, presets_len) = shared_cfg
                .read()
                .map(|c| (c.preset_cycle_secs, c.presets_order.len()))
                .unwrap_or((None, 0));
            if let Some(secs) = cycle_secs {
                if presets_len > 0 {
                    preset_timer += frame_dt_secs;
                    if preset_timer >= secs {
                        preset_timer = 0.0;
                        preset_idx = (preset_idx + 1) % presets_len;
                        if let Ok(mut w) = shared_cfg.write() {
                            let next_profile = w.presets_order.get(preset_idx).cloned();
                            w.active_profile = next_profile;
                            let resolved = w.clone().resolve_profile();
                            *w = resolved;
                            tracing::info!(preset_idx, "advanced to next preset profile");
                        }
                    }
                }
            }
        }

        // Drain key events.
        while let Ok(event) = key_rx.try_recv() {
            match event {
                KeyEvent::CycleSurface => {
                    surface_idx = (surface_idx + 1) % SURFACE_CYCLE.len();
                    let name = SURFACE_CYCLE[surface_idx];
                    tracing::info!(surface = name, "cycling surface");
                    surf = build_surface_by_name(name);
                    current_surface_name = name.to_string();
                    let (mv, mi) = surf.mesh_vertices(40, 40);
                    renderer.update_surface_mesh(&mv, &mi);
                    // Respawn geodesics on the new surface.
                    geodesics.clear();
                    trails.clear();
                    let (tl, ng, fp, cm) = shared_cfg
                        .read()
                        .map(|c| (c.trail_length, c.num_geodesics, c.effective_fade_power(), c.color_mode.clone()))
                        .unwrap_or((300, 30, 2.0, "cycle".into()));
                    for i in 0..ng {
                        let (u, v) = surf.random_position(&mut rng);
                        let (du, dv) = surf.random_tangent(u, v, &mut rng);
                        let ci = pick_color_idx(i, colors.len(), &cm, &mut rng);
                        geodesics.push(Geodesic::new(u, v, du, dv, tl, ci));
                        trails.push(TrailBuffer::new(tl, colors[ci], fp));
                    }
                }
                KeyEvent::SpeedUp => {
                    if let Ok(mut c) = shared_cfg.write() {
                        c.rotation_speed *= 1.1;
                    }
                }
                KeyEvent::SpeedDown => {
                    if let Ok(mut c) = shared_cfg.write() {
                        c.rotation_speed *= 0.9;
                    }
                }
                KeyEvent::ResetGeodesics => {
                    let (tl, ng, fp, cm) = shared_cfg
                        .read()
                        .map(|c| (c.trail_length, c.num_geodesics, c.effective_fade_power(), c.color_mode.clone()))
                        .unwrap_or((300, 30, 2.0, "cycle".into()));
                    geodesics.clear();
                    trails.clear();
                    for i in 0..ng {
                        let (u, v) = surf.random_position(&mut rng);
                        let (du, dv) = surf.random_tangent(u, v, &mut rng);
                        let ci = pick_color_idx(i, colors.len(), &cm, &mut rng);
                        geodesics.push(Geodesic::new(u, v, du, dv, tl, ci));
                        trails.push(TrailBuffer::new(tl, colors[ci], fp));
                    }
                    tracing::info!("geodesics reset");
                }
                KeyEvent::ToggleFpsHud => {
                    renderer.toggle_fps_hud();
                    tracing::info!(show_fps_hud = renderer.show_fps_hud, "toggled FPS HUD");
                }
            }
        }

        // Check for config reload flash.
        if reload_rx.try_recv().is_ok() {
            reload_flash_timer = 0.5;
        }

        // Read current config snapshot.
        let (rot, bg_color_str, cycle_enabled, cycle_speed, light_dir_cfg) =
            shared_cfg.read().map(|c| {
                (
                    c.rotation_speed,
                    c.background_color.clone(),
                    c.color_cycle_enabled,
                    c.color_cycle_speed,
                    c.light_dir,
                )
            }).unwrap_or((0.001047, "#050510".into(), false, 0.0, [1.0, 1.0, 1.0]));

        // Update light direction.
        renderer.light_dir = light_dir_cfg;

        // Compute base background color.
        let (base_r, base_g, base_b) = parse_bg_color(&bg_color_str);

        // Apply reload flash or normal background.
        if reload_flash_timer > 0.0 {
            reload_flash_timer -= frame_dt_secs;
            let t = (reload_flash_timer / 0.5).min(1.0).max(0.0);
            let flash_r = base_r * (1.0 - t as f64) + 0.3 * t as f64;
            let flash_g = base_g * (1.0 - t as f64) + 0.3 * t as f64;
            let flash_b = base_b * (1.0 - t as f64) + 0.8 * t as f64;
            renderer.set_background(flash_r, flash_g, flash_b);
        } else {
            renderer.set_background(base_r, base_g, base_b);
        }

        // Orbit camera (azimuth) and drift elevation.
        renderer.camera.orbit(rot * frame_dt_secs);
        renderer.camera.drift_elevation(frame_dt_secs);

        // Update effective colors (may change due to gradient or hue cycle).
        colors = if let Ok(c) = shared_cfg.read() {
            c.effective_colors()
        } else {
            colors.clone()
        };

        // Apply hue rotation if enabled.
        let display_colors = if cycle_enabled && cycle_speed > 0.0 {
            let degrees = elapsed_secs * cycle_speed * 360.0 % 360.0;
            rotate_hue(&colors, degrees)
        } else {
            colors.clone()
        };

        // Step geodesics in parallel using rayon.
        {
            let surf_ref = &*surf;
            geodesics
                .par_iter_mut()
                .zip(trails.par_iter_mut())
                .for_each(|(geo, trail)| {
                    if !geo.alive {
                        return;
                    }
                    let pos = surf_ref.position(geo.u, geo.v);
                    trail.push([pos.x, pos.y, pos.z]);
                    geo.step(surf_ref, dt);
                });
        }

        // Respawn dead geodesics sequentially (needs mutable rng).
        let (tl, cm, fp) = shared_cfg
            .read()
            .map(|c| (c.trail_length, c.color_mode.clone(), c.effective_fade_power()))
            .unwrap_or((300, "cycle".into(), 2.0));

        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive {
                let (u, v) = surf.random_position(&mut rng);
                let (du, dv) = surf.random_tangent(u, v, &mut rng);
                let ci = pick_color_idx(i, display_colors.len(), &cm, &mut rng);
                *geo = Geodesic::new(u, v, du, dv, tl, ci);
                trails[i].clear();
                trails[i].color = display_colors[ci % display_colors.len().max(1)];
                trails[i].fade_power = fp;
            }
        }

        // Collect trail vertices, applying display colors.
        let mut all_verts = Vec::new();
        let mut seg_lens = Vec::new();
        for (trail_idx, trail) in trails.iter().enumerate() {
            let ci = geodesics[trail_idx].color_idx % display_colors.len().max(1);
            let [dr, dg, db, _] = display_colors[ci];
            let mut v = trail.ordered_vertices();
            // Override the RGB with the display (possibly hue-rotated) color,
            // while preserving the alpha computed by ordered_vertices.
            for vert in &mut v {
                vert.color[0] = dr;
                vert.color[1] = dg;
                vert.color[2] = db;
            }
            seg_lens.push(v.len());
            all_verts.extend(v);
        }

        renderer.render(&all_verts, &seg_lens);
    }
}
