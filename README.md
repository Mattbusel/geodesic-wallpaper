# Geodesic Flow -- Live Desktop Wallpaper

[![CI](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/ci.yml/badge.svg)](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/ci.yml)
[![Release](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/release.yml/badge.svg)](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

A real-time animated desktop wallpaper for Windows that renders families of
geodesics flowing across parameterized surfaces. Geodesics are integrated with
a fourth-order Runge-Kutta (RK4) scheme using analytically computed Christoffel
symbols. The window sits below all application windows via a Win32
WM_WINDOWPOSCHANGING hook, leaving desktop icons fully accessible.

Built with Rust, wgpu (DirectX 12 / Vulkan / Metal), and the windows crate for
Win32 integration.

---

## Download

Head to the [Releases](../../releases/latest) page and download
`geodesic-wallpaper-windows.zip`. Extract it alongside `config.toml` and
run `geodesic-wallpaper.exe`. No installer required.

**Requirements:** Windows 10 or 11, any GPU with DirectX 12 or Vulkan support.

---

## Architecture

| Module | File | Responsibility |
|--------|------|----------------|
| `config` | `src/config.rs` | Load and hot-reload `config.toml`; parse CSS hex colours |
| `error` | `src/error.rs` | Typed error enum covering all subsystems |
| `surface` | `src/surface/` | `Surface` trait + `Torus`, `Sphere`, `Saddle` implementations |
| `geodesic` | `src/geodesic.rs` | RK4 integrator for the geodesic ODE using Christoffel symbols |
| `trail` | `src/trail.rs` | Fixed-capacity ring buffer with quadratic alpha fade |
| `renderer` | `src/renderer/` | wgpu render pipelines (surface wireframe + trail lines) |
| `renderer::camera` | `src/renderer/camera.rs` | Orbiting perspective camera |
| `wallpaper` | `src/wallpaper.rs` | Win32 borderless window pinned below all app windows |
| `main` | `src/main.rs` | Application entry point, message loop, hot-reload watcher |

```
config.toml
    |
    v
+--------+     hot-reload      +------------------+
| Config | -----------------> | SharedConfig      |
+--------+   (notify watcher) | Arc<RwLock<...>>  |
                               +------------------+
                                        |
                                        v
                        +-----------------------------+
                        | GeodesicEngine              |
                        | - Surface trait             |
                        |   (Torus / Sphere / Saddle) |
                        | - Geodesic: RK4 integrator  |
                        | - Christoffel symbols       |
                        +-----------------------------+
                                        |
                                        v
                        +-----------------------------+
                        | TrailRenderer               |
                        | - TrailBuffer (ring buffer) |
                        | - quadratic alpha fade      |
                        +-----------------------------+
                                        |
                                        v
                              +-----------------+
                              | wgpu pipelines  |
                              | (surface mesh + |
                              |  trail lines)   |
                              +-----------------+
                                        |
                                        v
                        +-------------------------------+
                        | Win32 WorkerW window          |
                        | (HWND_BOTTOM, WS_POPUP,       |
                        |  WM_WINDOWPOSCHANGING locked) |
                        +-------------------------------+
```

---

## Building from source

**Requirements:** Rust stable (1.75 or later), Windows 10 or 11, a GPU with
DirectX 12, Vulkan, or Metal support.

```powershell
git clone https://github.com/Mattbusel/geodesic-wallpaper.git
cd geodesic-wallpaper
cargo build --release
.\target\release\geodesic-wallpaper.exe
```

Place `config.toml` in the same directory as the executable. The application
reloads the file automatically whenever it changes on disk. Press Ctrl+C in the
terminal or close the process to stop.

To run tests (no GPU required):

```powershell
cargo test --lib
```

---

## Configuration reference

All fields are optional. Missing fields revert to the defaults listed below.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `surface` | string | `"torus"` | Surface to render: `"torus"`, `"sphere"`, or `"saddle"` |
| `num_geodesics` | integer | `30` | Number of simultaneous geodesic curves |
| `trail_length` | integer | `300` | Frames a trail persists before respawning |
| `rotation_speed` | float | `0.001047` | Camera orbit speed in radians per second |
| `color_palette` | string[] | 5 entries | CSS hex colour strings cycled across geodesics |
| `torus_R` | float | `2.0` | Torus major radius (centre to tube centre) |
| `torus_r` | float | `0.7` | Torus minor radius (tube radius) |
| `time_step` | float | `0.016` | RK4 integration timestep in seconds per frame |

```toml
# Surface to render.
# Accepted values: "torus", "sphere", "saddle".
# Any other value falls back to "torus".
# Default: "torus"
surface = "torus"

# Number of simultaneous geodesic curves.
# Default: 30
num_geodesics = 30

# Number of frames a trail persists before being respawned.
# Default: 300
trail_length = 300

# Camera orbit angular speed in radians per second.
# 0.001047 is approximately one full revolution every 100 minutes.
# Default: 0.001047
rotation_speed = 0.001047

# Trail color palette as CSS hex strings (with or without leading '#').
# Geodesics cycle through the list in order.
# Default: five entries from blue to gold to pink.
color_palette = ["#4488FF", "#88DDFF", "#FFD700", "#88FF88", "#FF88CC"]

# Torus major radius: distance from the center of the torus to the center
# of the tube.
# Default: 2.0
torus_R = 2.0

# Torus minor radius: radius of the tube.
# Default: 0.7
torus_r = 0.7

# RK4 integration time step in seconds per frame.
# Values above 0.02 can cause visible trajectory drift on a small torus.
# Default: 0.016
time_step = 0.016
```

---

## How it works

A geodesic on a Riemannian surface is a curve whose acceleration has no
tangential component; equivalently, it satisfies the geodesic equation
d^2x^k/dt^2 + Gamma^k_ij (dx^i/dt)(dx^j/dt) = 0, where Gamma^k_ij are the
Christoffel symbols of the metric. Each symbol is computed analytically from
the first and second fundamental forms of the surface, avoiding finite-difference
approximations and the numerical noise they introduce. For the torus the metric
is diagonal (the parameterization is orthogonal), which collapses the Christoffel
computation to two non-zero components. The sphere and saddle use the full
symmetric formula.

The ODE is advanced with a classical fourth-order Runge-Kutta integrator. At
each frame the integrator evaluates the Christoffel symbols at four intermediate
positions and combines the results with the standard 1/6-2/6-2/6-1/6 weights.
After each step the velocity is renormalized to unit metric speed
(g_ij du^i du^j = 1) to suppress the slow drift caused by floating-point
truncation over hundreds of frames. This is correct because the geodesic
equation preserves metric speed exactly; renormalization merely restores the
invariant that the integrator breaks weakly.

Each geodesic leaves a trail stored in a fixed-capacity ring buffer. When a new
position is pushed, the oldest is silently overwritten. At render time the buffer
is iterated from oldest to newest and each vertex is assigned an alpha value
equal to (i / N)^2, producing a quadratic fade from transparent at the tail to
opaque at the head. The ring buffer never grows or reallocates after
initialization, so the per-frame allocation pressure is zero.

---

## Supported surfaces

| Surface | Curvature | Behavior |
|---------|-----------|----------|
| torus   | Mixed (positive outer rim, negative inner rim) | Geodesics diverge on the inner rim and focus on the outer |
| sphere  | Constant positive | All geodesics are great circles |
| saddle  | Constant negative | Geodesics diverge exponentially |

---

## Lua Custom Surfaces

When built with the optional `lua` feature flag, you can define entirely custom
Riemannian surfaces by providing a Lua script.

### Enable the feature

```powershell
cargo build --release --features lua
```

### Write a Lua script

Create a file such as `my_surface.lua`:

```lua
-- Metric tensor components at parameter position (u, v).
-- All four components are required.
function metric(u, v)
  return {
    g_uu = 1.0,
    g_uv = 0.0,
    g_vu = 0.0,
    g_vv = math.sin(u)^2,
  }
end

-- Optional: return Christoffel symbols directly.
-- If absent they are derived numerically via finite differences.
function christoffel(u, v)
  return {
    g000=0.0, g001=0.0, g010=0.0, g011=0.0,
    g100=0.0, g101=0.0, g110=0.0, g111=0.0,
  }
end
```

### Point config.toml at the script

```toml
surface    = "lua"
lua_script = "my_surface.lua"
```

The script is hot-reloaded whenever `config.toml` changes. If the Lua script
produces an error or a degenerate metric, the surface automatically falls back
to the default torus and a warning is printed to the console.

---

## Live Parameter Tuning

Individual parameters can be adjusted while the wallpaper is running using
keyboard shortcuts.  Declare tunable parameters in the `[tuning]` section of
`config.toml`:

```toml
[tuning]
[[tuning.parameters]]
name    = "rotation_speed"
min     = 0.0
max     = 0.1
current = 0.001047
step    = 0.0001

[[tuning.parameters]]
name    = "trail_fade_power"
min     = 0.5
max     = 5.0
current = 2.0
step    = 0.1
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `[` | Select previous parameter |
| `]` | Select next parameter |
| `-` | Decrease selected value by one step |
| `=` | Increase selected value by one step |

The current parameter name and value are shown as an overlay on the wallpaper.
When the application exits, updated values are written back to `config.toml` so
they persist across restarts.

---

## Phase Portrait Recording

Press **Shift+R** to start recording the geodesic animation. A sequence of PNG
frames is saved to a temporary directory.  When recording stops (either because
the time limit was reached or you press Shift+R again), the frames are assembled
into a GIF and saved to `geodesic-recording.gif` in the current directory.

Default settings: 10 seconds at 30 fps (300 frames). The window title shows
recording status: `[REC 3s / 10s  90 frames]`.

The recorder API is also available programmatically:

```rust
use geodesic_wallpaper::recorder::PhasePortraitRecorder;
use std::path::PathBuf;

let mut rec = PhasePortraitRecorder::new(1920, 1080, 30, 10,
    PathBuf::from("my-recording.gif"));
rec.start()?;
// push RGBA frames from the render loop…
rec.push_frame(&rgba_bytes)?;
let output_path = rec.finish()?;
```

---

## Gallery Mode

Gallery mode cycles through all available mathematical surfaces automatically,
displaying each one for a configurable duration with a smooth cross-fade
transition between surfaces.

### Enable via config.toml

```toml
gallery_mode       = true
gallery_duration_s = 30     # seconds per surface (default: 30)
```

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `G` | Toggle gallery mode on/off |
| `LEFT` | Skip to the previous surface |
| `RIGHT` | Skip to the next surface |

While gallery mode is active, the fade-out/fade-in transition takes
approximately 0.75 seconds each way, giving a 1.5-second crossfade between
surfaces.

---

## Contributing

1. Fork the repository.
2. Create a feature branch: `git checkout -b my-feature`.
3. Ensure `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` all pass.
4. Open a pull request against `master`.

CI enforces formatting, Clippy warnings-as-errors, the full test suite, and a
release build before merging.

---

## License

MIT -- see [LICENSE](LICENSE) for details.
