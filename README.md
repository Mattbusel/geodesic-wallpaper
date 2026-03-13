# Geodesic Flow — Live Desktop Wallpaper

A real-time animated Windows desktop wallpaper that renders families of **geodesics** — the shortest paths on curved surfaces — flowing across a torus, sphere, or saddle, leaving luminous fading trails behind.

Built with **Rust**, **wgpu**, and proper differential geometry. Runs silently behind your desktop icons via the Win32 WorkerW trick.

---

## Preview

> Glowing geodesic trails spiral across a dark torus, their colors shifting from electric blue to soft gold as they trace paths dictated by the surface's curvature.

---

## Download (Windows .exe)

Head to the [**Releases**](../../releases/latest) page and download `geodesic-wallpaper.exe`. No install needed — just run it.

> **Requirements:** Windows 10/11, a GPU that supports DirectX 12, Vulkan, or Metal (any modern GPU works).

---

## What It Does

- Renders **N geodesics** (default 30) simultaneously on a parameterized surface embedded in 3D
- Each geodesic is integrated using **RK4** on the geodesic equation:

  ```
  d²xᵏ/dt² + Γᵏᵢⱼ (dxⁱ/dt)(dxʲ/dt) = 0
  ```

  where `Γᵏᵢⱼ` are the **Christoffel symbols** computed analytically from the surface's metric tensor

- Trails **fade quadratically** from bright (head) to transparent (tail) over a configurable number of frames
- Camera **slowly orbits** the surface (one full revolution every ~10 minutes by default)
- The window lives **behind your desktop icons** — no taskbar entry, no chrome, just the animation
- **Hot-reloads** `config.toml` while running — tweak parameters live

---

## Supported Surfaces

| Surface | Geometry | Behavior |
|---|---|---|
| `torus` | Mixed curvature | Geodesics diverge on the inner rim, converge on the outer |
| `sphere` | Constant positive | All geodesics are great circles |
| `saddle` | Negative curvature | Geodesics exponentially diverge |

---

## Getting Started

### Option A — Download the .exe (easiest)

1. Go to [Releases](../../releases/latest)
2. Download `geodesic-wallpaper.exe`
3. Place it in a folder alongside `config.toml` (see below)
4. Double-click to run — your desktop wallpaper activates instantly
5. Press `Ctrl+C` in the terminal or close the window to stop

### Option B — Build from source

**Prerequisites:** [Rust](https://rustup.rs/) (stable, 1.75+), Windows 10/11

```powershell
git clone https://github.com/Mattbusel/geodesic-wallpaper.git
cd geodesic-wallpaper
cargo run --release
```

---

## Configuration

Edit `config.toml` in the same directory as the exe. Changes apply **instantly** (hot-reload — no restart needed).

```toml
# Surface to render: "torus", "sphere", or "saddle"
surface = "torus"

# Number of simultaneous geodesics
num_geodesics = 30

# How many frames a trail persists before fading out
trail_length = 300

# Camera orbit speed in radians/second (0.001047 ≈ 1 rev per 10 min)
rotation_speed = 0.001047

# Trail colors (hex). Geodesics cycle through these.
color_palette = ["#4488FF", "#88DDFF", "#FFD700", "#88FF88", "#FF88CC"]

# Torus geometry
torus_R = 2.0   # major radius (center to tube center)
torus_r = 0.7   # minor radius (tube radius)
```

---

## Architecture

```
src/
├── main.rs              # winit app loop, frame limiter, geodesic respawn
├── wallpaper.rs         # Win32 Progman/WorkerW integration (all unsafe isolated here)
├── config.rs            # TOML config + file-watcher hot reload
├── geodesic.rs          # RK4 integrator for the geodesic ODE
├── trail.rs             # Ring buffer of trail vertices with fade
├── surface/
│   ├── mod.rs           # Surface trait (position, metric, Christoffel, mesh)
│   ├── torus.rs         # Analytic Christoffel symbols for torus
│   ├── sphere.rs        # Analytic Christoffel symbols for sphere
│   └── saddle.rs        # Saddle (hyperbolic paraboloid)
└── renderer/
    ├── mod.rs           # wgpu render pipelines, surface wireframe + trail draws
    ├── camera.rs        # Slowly orbiting perspective camera
    └── shaders/
        ├── surface.wgsl # Dim wireframe mesh
        └── trail.wgsl   # Glow-on-alpha trail lines
```

---

## Performance

Targeting **< 5% GPU utilization** at 1080p/30fps. This is a background wallpaper — it stays out of your way.

- Christoffel symbols computed analytically (no finite differences)
- Trail vertices in a **ring buffer** — no growing allocations
- Frame-limited to 30fps with `thread::sleep`
- Low-power GPU adapter preference via wgpu

---

## Tech Stack

- **Rust** — memory safety, zero-cost abstractions
- **wgpu** — cross-backend GPU rendering (DX12 / Vulkan / Metal)
- **winit** — cross-platform windowing
- **windows crate** — Win32 FFI for WorkerW desktop integration
- **glam** — fast f32 vector/matrix math
- **notify** — filesystem watcher for hot config reload

---

## License

MIT

---

#rust #wgpu #windows #desktop-wallpaper #geodesics #differential-geometry #riemannian-geometry #live-wallpaper #generative-art #math-visualization #graphics #gpu #shader #torus #winit
