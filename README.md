# Geodesic Flow -- Live Desktop Wallpaper

[![CI](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/ci.yml/badge.svg)](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/ci.yml)
[![Release](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/release.yml/badge.svg)](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

**geodesic-wallpaper** is a real-time animated desktop wallpaper for Windows that renders families of geodesic curves flowing across parameterised Riemannian surfaces. Curves are integrated with a fourth-order Runge-Kutta (RK4) scheme using analytically-computed Christoffel symbols for **fourteen built-in surfaces** (torus, sphere, saddle, catenoid, helicoid, hyperboloid, hyperbolic paraboloid, ellipsoid, Enneper, Klein bottle, Boy surface, torus knot, **pseudosphere**, **trefoil tube**). The window sits below all application windows via a Win32 WM_WINDOWPOSCHANGING hook so desktop icons remain fully accessible. Optional modules let you drive the animation from live financial market data and assign a different surface to each physical monitor.

---

## What does it look like?

```
          . . . * * * * . . .
       . *   __-------__   * .
      * /  /    ~~~~    \  \ *
     * |  | ((  .  .  )) |  | *
     . |  |  \\  \_/  //  |  | .
     * |  |   \\  |  //   |  | *
      * \  \    ``---''    /  / *
       . *   -----------   * .
          . . . * * * * . . .

  Geodesic curves flowing across a torus surface.
  Each coloured trail fades from opaque at the head
  to transparent at the tail (quadratic alpha decay).
```

Surfaces range from the familiar torus and sphere to exotic objects like the non-orientable Klein bottle, the self-intersecting Enneper minimal surface, and the trefoil torus-knot tube.

---

## 5-minute quickstart

### Binary (no build required)

1. Download `geodesic-wallpaper-windows.zip` from the [Releases](../../releases/latest) page.
2. Extract the ZIP — you will find `geodesic-wallpaper.exe` and a sample `config.toml`.
3. Double-click `geodesic-wallpaper.exe`, or run from a terminal:

```powershell
.\geodesic-wallpaper.exe
```

Requirements: Windows 10 or 11, any GPU with DirectX 12 or Vulkan support. No installer, no runtime dependencies.

### Quickstart with a named preset

```powershell
.\geodesic-wallpaper.exe --preset cosmic
.\geodesic-wallpaper.exe --preset ocean
.\geodesic-wallpaper.exe --preset ember
```

See the [Scene Presets](#scene-presets) section for the full list.

---

## Configuration reference

All fields are optional. Missing fields revert to the defaults shown.

Place `config.toml` in the same directory as the executable. The file is hot-reloaded automatically whenever it changes on disk — no restart required.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `surface` | string | `"torus"` | Surface to render. See [Supported Surfaces](#supported-surfaces). |
| `num_geodesics` | integer | `30` | Number of simultaneous geodesic curves. |
| `trail_length` | integer | `300` | Frames a trail persists before respawning. |
| `rotation_speed` | float | `0.001047` | Camera orbit speed in radians per second. |
| `color_palette` | string[] | 5 entries | CSS hex colour strings cycled across geodesics. |
| `torus_R` | float | `2.0` | Torus major radius (centre to tube centre). |
| `torus_r` | float | `0.7` | Torus minor radius (tube radius). |
| `time_step` | float | `0.016` | RK4 integration timestep in seconds per frame. |
| `catenoid_c` | float | `1.0` | Catenoid scale parameter. |
| `helicoid_c` | float | `1.0` | Helicoid pitch parameter. |
| `hyperboloid_a` | float | `1.0` | Hyperboloid semi-axis a. |
| `hyperboloid_b` | float | `1.0` | Hyperboloid semi-axis b. |
| `ellipsoid_a` | float | `2.0` | Ellipsoid semi-axis along x. |
| `ellipsoid_b` | float | `1.5` | Ellipsoid semi-axis along y. |
| `ellipsoid_c` | float | `1.0` | Ellipsoid semi-axis along z. |
| `hyperbolic_paraboloid_a` | float | `1.0` | Saddle+ semi-axis a. |
| `hyperbolic_paraboloid_b` | float | `1.0` | Saddle+ semi-axis b. |
| `camera_distance` | float | `6.0` | Camera distance from origin. |
| `camera_elevation` | float | `0.4` | Camera elevation in radians. |
| `camera_fov` | float | `0.8` | Vertical field-of-view in radians. |
| `show_wireframe` | bool | `true` | Render surface wireframe mesh. |
| `trail_fade_power` | float | `2.0` | Exponent for trail alpha fade (1=linear, 2=quadratic). |
| `target_fps` | integer | `30` | Target frame rate. |
| `background_color` | string | `"#050510"` | Background clear colour as CSS hex. |
| `color_mode` | string | `"cycle"` | `"cycle"` or `"random"` colour assignment. |
| `gallery_mode` | bool | `false` | Auto-cycle through all surfaces. |
| `gallery_duration_s` | integer | `30` | Seconds per surface in gallery mode. |
| `lua_script` | string | — | Path to a Lua surface script (requires `--features lua`). |

### Minimal config.toml example

```toml
surface          = "torus"
num_geodesics    = 30
trail_length     = 300
rotation_speed   = 0.001047
background_color = "#050510"
color_palette    = ["#4488FF", "#88DDFF", "#FFD700", "#88FF88", "#FF88CC"]
```

---

## Multi-monitor setup

When you have more than one physical monitor connected, `multi_monitor::MultiMonitorManager` assigns a different surface and colour scheme to each display automatically.

### How it works

At startup the manager enumerates monitors via Win32 `EnumDisplayMonitors`, then cycles through the twelve built-in surfaces in order — monitor 0 gets the torus, monitor 1 gets the sphere, and so on.

### Configure via `config.toml`

The simplest option is to launch with `--preset` per monitor from separate instances, or define per-monitor overrides programmatically:

```rust
use geodesic_wallpaper::multi_monitor::{MonitorConfig, MultiMonitorManager};

let manager = MultiMonitorManager::from_config(vec![
    MonitorConfig {
        monitor_index: 0,
        surface: "torus".into(),
        color_scheme: "cosmic".into(),
        geodesic_count: 35,
        speed: 1.0,
    },
    MonitorConfig {
        monitor_index: 1,
        surface: "sphere".into(),
        color_scheme: "ocean".into(),
        geodesic_count: 25,
        speed: 0.8,
    },
]);

println!("{} monitors configured", manager.monitor_count());
```

### Defaults

If no explicit configuration is provided, `MultiMonitorManager::new()` auto-assigns surfaces:

| Monitor | Surface | Colour scheme |
|---------|---------|---------------|
| 0 | torus | cosmic |
| 1 | sphere | ocean |
| 2 | saddle | ember |
| 3 | hyperboloid | forest |
| 4 | klein\_bottle | aurora |
| 5+ | cycles back | cycles back |

---

## Financial data driver

The `finance_driver::FinanceDriver` maps OHLCV market data to geodesic animation parameters in real time.

| Market signal | Geodesic parameter |
|---------------|--------------------|
| Realized volatility (stddev of log-returns) | `speed_multiplier` — geodesics accelerate in volatile markets |
| Trend direction (OLS slope of close prices) | `theta_velocity` — positive in uptrends, negative in downtrends |
| Normalized volume | `trail_width` and `phi_velocity` |
| Trend mapped to \[0, 0.33\] | `color_hue` — red in downtrends, green in uptrends |

### Push bars manually

```rust
use geodesic_wallpaper::finance_driver::{FinanceDriver, MarketBar};

let mut driver = FinanceDriver::new(20); // 20-bar rolling window

driver.push_bar(MarketBar {
    open: 150.00, high: 152.30, low: 149.10, close: 151.80,
    volume: 1_234_567.0, timestamp: 1_700_000_000,
});

let params = driver.compute_params();
println!("Speed multiplier: {:.3}", params.speed_multiplier);
println!("Trail width: {:.1}px", params.trail_width);
println!("Color hue: {:.3}", params.color_hue);
```

### Load from CSV

The CSV format is `timestamp,open,high,low,close,volume` with no header row. Lines starting with `#` are treated as comments.

```rust
use geodesic_wallpaper::finance_driver::FinanceDriver;

let csv = std::fs::read_to_string("prices.csv").unwrap();
let mut driver = FinanceDriver::new(50);
let loaded = driver.load_csv(&csv);
println!("Loaded {loaded} bars");

let params = driver.compute_params();
```

Sample `prices.csv`:

```csv
# timestamp,open,high,low,close,volume
1700000000,100.00,102.50,99.20,101.75,980000
1700000060,101.75,103.00,101.00,102.40,1150000
1700000120,102.40,102.80,100.50,100.90,870000
```

---

## Scene presets

The `scene_presets::PresetLibrary` ships with 10 built-in named configurations.

| Name | Surface | Mood |
|------|---------|------|
| `cosmic` | torus | Deep-space: gold geodesics through an indigo void |
| `ocean` | sphere | Ocean depths: cyan great-circle flows |
| `ember` | saddle | Volcanic: orange-red streaks across a hyperbolic saddle |
| `forest` | hyperboloid | Canopy: lush greens on a one-sheeted hyperboloid |
| `aurora` | torus\_knot | Arctic: shimmering green-blue trefoil traces |
| `neon` | klein\_bottle | City night: hot pink and electric blue on the Klein bottle |
| `dusk` | ellipsoid | Twilight: rose-to-purple sweeps |
| `ice` | catenoid | Crystal cave: steel-blue spirals around the catenoid waist |
| `lava` | boy\_surface | Volcanic: molten yellow-red on Boy's surface |
| `void` | enneper | Deep nothing: slow teal traces on the Enneper surface |

### Use from command line

```powershell
.\geodesic-wallpaper.exe --preset aurora
.\geodesic-wallpaper.exe --preset neon
```

### Use from code

```rust
use geodesic_wallpaper::scene_presets::PresetLibrary;

let mut lib = PresetLibrary::with_defaults();

// Navigate presets.
let next = lib.next();
println!("Switched to: {} — {}", next.name, next.description);

// Look up by name.
if let Some(p) = lib.by_name("cosmic") {
    println!("Background: {}", p.background_hex());
}
```

---

## Supported surfaces

All surfaces implement the `Surface` trait: `position()`, `normal()`, `metric()`, `christoffel()`, `wrap()`, `random_position()`, `random_tangent()`, and `mesh_vertices()`.

| Surface | Config name | Curvature | Notes |
|---------|-------------|-----------|-------|
| Torus | `"torus"` | Mixed | Analytic Christoffels; ergodic irrational windings |
| Sphere | `"sphere"` | Constant positive K = 1/R² | All geodesics are great circles |
| Saddle | `"saddle"` | Zero (flat chart) | Straight-line geodesics |
| Catenoid | `"catenoid"` | Negative (minimal) | Geodesics spiral around the waist |
| Helicoid | `"helicoid"` | Negative (minimal) | Isometric to catenoid |
| Hyperboloid | `"hyperboloid"` | Negative | One-sheeted ruled quadric |
| Hyperbolic paraboloid | `"hyperbolic_paraboloid"` | Negative | Doubly ruled; z = u²/a² − v²/b² |
| Ellipsoid | `"ellipsoid"` | Positive (varying) | Three independent semi-axes |
| Enneper | `"enneper"` | Negative (minimal) | Complete; total curvature −4π; self-intersects |
| Klein bottle | `"klein_bottle"` | Non-orientable | Figure-8 immersion in ℝ³ |
| Boy's surface | `"boy_surface"` | Non-orientable | RP² with 3-fold symmetry (Apery form) |
| Torus knot | `"torus_knot"` | Positive (tube) | Default T(2,3) trefoil |
| **Pseudosphere** | `"pseudosphere"` | **Constant negative K = −1** | Tractricoid; geodesics diverge exponentially — the hyperbolic plane's classic model surface |
| **Trefoil tube** | `"trefoil"` | Positive (tube) | Circular cross-section swept around the trefoil knot curve; geodesics precess across all three lobes |

---

## Mouse and keyboard controls

### Keyboard

| Key | Action |
|-----|--------|
| `S` | Cycle to next surface |
| `G` | Toggle gallery mode |
| `LEFT` | Previous surface (gallery mode) |
| `RIGHT` | Next surface (gallery mode) |
| `+` / `=` | Increase rotation speed ×1.1 |
| `-` | Decrease rotation speed ×0.9 |
| `R` | Reset all geodesics |
| `H` | Toggle FPS HUD overlay |
| `[` | Select previous tunable parameter |
| `]` | Select next tunable parameter |
| `Shift+R` | Start / stop phase portrait recording |

### Mouse

| Action | Effect |
|--------|--------|
| Left-click | Shoot a new geodesic from the clicked surface point |
| Right-click | Remove all geodesics and reset |
| Middle-click | Cycle to the next surface |
| Scroll up | Increase geodesic speed ×1.1 per notch |
| Scroll down | Decrease geodesic speed ×0.9 per notch |

---

## Gallery mode

Gallery mode cycles through all surfaces automatically with a smooth cross-fade transition.

```toml
gallery_mode       = true
gallery_duration_s = 30
```

---

## Lua custom surfaces

Build with `--features lua` to define entirely custom Riemannian surfaces in Lua.

```powershell
cargo build --release --features lua
```

```lua
-- my_surface.lua
function metric(u, v)
  return { g_uu=1.0, g_uv=0.0, g_vu=0.0, g_vv=math.sin(u)^2 }
end
```

```toml
surface    = "lua"
lua_script = "my_surface.lua"
```

The script is hot-reloaded whenever `config.toml` changes. Invalid scripts fall back to the torus.

---

## Live parameter tuning

```toml
[tuning]
[[tuning.parameters]]
name    = "rotation_speed"
min     = 0.0
max     = 0.1
current = 0.001047
step    = 0.0001
```

Use `[` / `]` to select a parameter and `-` / `=` to adjust its value live. Updated values are written back to `config.toml` on exit.

---

## Phase portrait recording

Press **Shift+R** to record. Frames are saved as PNGs and assembled into `geodesic-recording.gif` when recording stops. Default: 10 seconds at 30 fps.

---

## Architecture

| Module | Responsibility |
|--------|----------------|
| `config` | Load and hot-reload `config.toml`; parse CSS hex colours |
| `error` | Typed error enum covering all subsystems |
| `surface` | `Surface` trait + 12 implementations |
| `geodesic` | RK4 integrator for the geodesic ODE |
| `interactive` | Mouse event handling; geodesic shooting |
| `trail` | Fixed-capacity ring buffer with quadratic alpha fade |
| `renderer` | wgpu render pipelines (surface wireframe + trail lines) |
| `wallpaper` | Win32 borderless window pinned below all app windows |
| `gallery` | Auto-cycle through surfaces in gallery mode |
| `parameter_tuner` | Runtime keyboard-driven parameter adjustment |
| `recorder` | Phase portrait PNG/GIF recording |
| `multi_monitor` | Per-monitor surface assignment and configuration |
| `finance_driver` | Maps OHLCV market data to geodesic parameters |
| `scene_presets` | Named scene configurations with sequential navigation |
| `main` | Application entry point, message loop, hot-reload watcher |

---

## Building from source

Requirements: Rust stable 1.75+, Windows 10 or 11, GPU with DirectX 12 or Vulkan support.

```powershell
git clone https://github.com/Mattbusel/geodesic-wallpaper.git
cd geodesic-wallpaper
cargo build --release
.\target\release\geodesic-wallpaper.exe
```

To run tests (no GPU required):

```powershell
cargo test --lib
```

To build with Lua scripting support:

```powershell
cargo build --release --features lua
```

---

## Contributing

1. Fork the repository.
2. Create a feature branch: `git checkout -b my-feature`.
3. Ensure `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --lib` all pass.
4. Open a pull request against `main`.

CI enforces formatting, Clippy warnings-as-errors, the full test suite, and a release build before merging.

---

## License

MIT — see [LICENSE](LICENSE) for details.
