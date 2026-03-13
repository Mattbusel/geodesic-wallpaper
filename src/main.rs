mod config;
mod surface;
mod geodesic;
mod trail;
mod renderer;
mod wallpaper;

use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId, WindowAttributes};
use winit::platform::windows::WindowAttributesExtWindows;
use rand::SeedableRng;
use rand::rngs::StdRng;

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

struct App {
    shared_cfg: SharedConfig,
    config_path: PathBuf,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    surf: Box<dyn Surface>,
    colors: Vec<[f32; 4]>,
    geodesics: Vec<Geodesic>,
    trails: Vec<TrailBuffer>,
    rng: StdRng,
    last_frame: std::time::Instant,
    total_time: f32,
    dt: f32,
    target_frame_time: std::time::Duration,
}

impl App {
    fn new(cfg: Config, shared_cfg: SharedConfig, config_path: PathBuf) -> Self {
        let surf = build_surface(&cfg);
        let colors = parse_colors(&cfg);
        let mut rng = StdRng::from_entropy();
        let mut geodesics = Vec::new();
        let mut trails = Vec::new();

        for i in 0..cfg.num_geodesics {
            let (u, v) = surf.random_position(&mut rng);
            let (du, dv) = surf.random_tangent(u, v, &mut rng);
            let color_idx = i % colors.len();
            geodesics.push(Geodesic::new(u, v, du, dv, cfg.trail_length, color_idx));
            trails.push(TrailBuffer::new(cfg.trail_length, colors[color_idx]));
        }

        Self {
            shared_cfg,
            config_path,
            window: None,
            renderer: None,
            surf,
            colors,
            geodesics,
            trails,
            rng,
            last_frame: std::time::Instant::now(),
            total_time: 0.0,
            dt: 0.04,
            target_frame_time: std::time::Duration::from_millis(33),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let monitor_size = event_loop.primary_monitor()
            .map(|m| m.size())
            .unwrap_or(winit::dpi::PhysicalSize::new(1920, 1080));

        let attrs = WindowAttributes::default()
            .with_title("Geodesic Wallpaper")
            .with_inner_size(monitor_size)
            .with_decorations(false)
            .with_skip_taskbar(true)
            .with_position(winit::dpi::PhysicalPosition::new(0, 0));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());

        // Attempt WorkerW attachment
        {
            use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
            use windows::Win32::Foundation::HWND;
            if let Ok(handle) = window.window_handle() {
                if let RawWindowHandle::Win32(h) = handle.as_raw() {
                    let hwnd = HWND(h.hwnd.get() as _);
                    wallpaper::attach_to_desktop(hwnd);
                }
            }
        }

        let (mesh_verts, mesh_indices) = self.surf.mesh_vertices(40, 40);
        let renderer = pollster::block_on(Renderer::new(
            window.clone(),
            &mesh_verts,
            &mesh_indices,
        ));

        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(r) = &mut self.renderer {
                    r.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(self.last_frame);
                if elapsed < self.target_frame_time {
                    std::thread::sleep(self.target_frame_time - elapsed);
                }
                self.last_frame = std::time::Instant::now();
                self.total_time += self.dt;

                let rotation_speed = self.shared_cfg.read()
                    .map(|c| c.rotation_speed)
                    .unwrap_or(0.001047);

                if let Some(r) = &mut self.renderer {
                    r.camera.orbit(rotation_speed * self.dt);
                }

                // Step geodesics, push trail points
                for (i, geo) in self.geodesics.iter_mut().enumerate() {
                    if !geo.alive { continue; }
                    let pos = self.surf.position(geo.u, geo.v);
                    self.trails[i].push([pos.x, pos.y, pos.z]);
                    geo.step(self.surf.as_ref(), self.dt);
                }

                // Respawn dead geodesics
                for (i, geo) in self.geodesics.iter_mut().enumerate() {
                    if !geo.alive {
                        let (u, v) = self.surf.random_position(&mut self.rng);
                        let (du, dv) = self.surf.random_tangent(u, v, &mut self.rng);
                        let trail_length = self.shared_cfg.read()
                            .map(|c| c.trail_length)
                            .unwrap_or(300);
                        let color_idx = i % self.colors.len();
                        *geo = Geodesic::new(u, v, du, dv, trail_length, color_idx);
                        self.trails[i].clear();
                        self.trails[i].color = self.colors[color_idx];
                    }
                }

                // Collect all trail vertices
                let mut all_verts = Vec::new();
                let mut segment_lens = Vec::new();
                for trail in &self.trails {
                    let verts = trail.ordered_vertices();
                    let len = verts.len();
                    all_verts.extend(verts);
                    segment_lens.push(len);
                }

                if let Some(r) = &mut self.renderer {
                    r.render(&all_verts, &segment_lens);
                }

                // Request next frame
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config_path = PathBuf::from("config.toml");
    let cfg = Config::load(&config_path);
    let shared_cfg: SharedConfig = Arc::new(RwLock::new(cfg.clone()));

    // Set up file watcher for hot reload
    {
        let shared = shared_cfg.clone();
        let path = config_path.clone();
        std::thread::spawn(move || {
            use notify::{Watcher, RecursiveMode, recommended_watcher};
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = recommended_watcher(move |res| {
                let _ = tx.send(res);
            }).unwrap();
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

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(cfg, shared_cfg, config_path);
    event_loop.run_app(&mut app).unwrap();
}
