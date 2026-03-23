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

Gallery mode cycles through all fourteen built-in surfaces automatically with a smooth cross-fade transition. It acts as a mathematical screensaver, showing each surface for a configurable interval before smoothly transitioning to the next.

### Enable in `config.toml`

```toml
gallery_mode       = true
gallery_duration_s = 30   # seconds per surface (default 30, minimum 1)
```

### CLI flag

```powershell
.\geodesic-wallpaper.exe --gallery
```

### Keyboard controls (gallery mode)

| Key | Action |
|-----|--------|
| `G` | Toggle gallery mode on / off |
| `RIGHT` / `SPACE` | Skip to next surface immediately |
| `LEFT` | Skip to previous surface |

### Transition

Each surface change is accompanied by a brief cross-fade (0.75 s fade-out, 0.75 s fade-in). The caller reads `GalleryMode::transition_alpha()` each frame and multiplies scene opacity accordingly.

### Surface cycle order

```
torus → sphere → saddle → enneper → catenoid → helicoid →
hyperboloid → hyperbolic_paraboloid → ellipsoid → (wraps)
```

### Use from code

```rust
use geodesic_wallpaper::gallery::GalleryMode;

// Create, enabled from the start, 45 seconds per surface.
let mut gallery = GalleryMode::new(true, 45);

// In the render loop:
let surface_changed = gallery.update();
let alpha = gallery.transition_alpha();  // multiply scene opacity by this
if surface_changed {
    let name = gallery.current_surface();
    println!("Now showing: {name}");
    // switch the renderer to this surface
}
```

### `GalleryConfig` struct

```rust
pub struct GalleryConfig {
    pub surfaces: Vec<SurfaceKind>,   // subset to cycle (default: all 14)
    pub interval_secs: f32,           // seconds per surface
    pub transition_secs: f32,         // fade half-duration
}
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

## Geodesic field visualization

Instead of rendering a fixed set of individual geodesic trails, the **geodesic field** mode fills the entire parameter domain `(u, v)` with a dense grid of small coloured arrows — one per grid cell — each pointing in the geodesic direction and coloured by the long-term fate of that geodesic.

### Basin classification

Each grid cell seeds a geodesic and integrates it for `fate_steps` RK4 steps. The final state determines the colour:

| Fate | Condition | Arrow colour |
|------|-----------|-------------|
| `Bounded` | Stays within `escape_radius` for all steps | Blue |
| `Escaping` | Exceeds `escape_radius` | Red |
| `Looping` | Returns within `loop_epsilon` of start | Green |

The resulting image reveals the **basin structure** of geodesic flow: which starting directions lead to bounded wandering, which ones escape the parameterisation domain, and which form closed loops.

### `FieldConfig`

```rust
pub struct FieldConfig {
    pub grid_n: usize,          // grid cells per axis (default 40, total = 40²= 1600)
    pub u_range: [f64; 2],      // parameter u range (default [-π, π])
    pub v_range: [f64; 2],      // parameter v range (default [-π, π])
    pub fate_steps: usize,       // integration steps per cell (default 200)
    pub fate_dt: f64,            // RK4 timestep for fate integration (default 0.04)
    pub escape_radius: f64,      // escape threshold (default 8.0)
    pub loop_epsilon: f64,       // loop detection radius (default 0.3)
    pub arrow_length: f32,       // arrow display length (default 0.04)
    pub arrow_alpha: f32,        // arrow opacity (default 0.7)
}
```

### Usage

```rust
use geodesic_wallpaper::field::{FieldConfig, FieldRenderer};

let cfg = FieldConfig { grid_n: 60, ..Default::default() };
let renderer = FieldRenderer::new(cfg);

// Provide your RK4 geodesic step function.
let field = renderer.compute(|u, v, du, dv| {
    // one step of the geodesic ODE on your surface
    (new_u, new_v, new_du, new_dv)
});

println!("{} arrows, {} bounded, {} escaping, {} looping",
    field.arrows.len(),
    field.fate_counts[0], field.fate_counts[1], field.fate_counts[2]);

// Upload field.arrows as an instanced vertex buffer for GPU rendering.
```

The computation is parallelised with rayon across all `grid_n²` cells, so a 60×60 grid (3 600 cells × 200 steps) completes in well under a second on a modern CPU.

### `FlowArrow` GPU instance layout

Each `FlowArrow` is `repr(C)` and implements `bytemuck::Pod`:

```rust
pub struct FlowArrow {
    pub origin:    [f32; 2],   // (u, v) base position
    pub direction: [f32; 2],   // normalised direction vector
    pub length:    f32,        // display length
    pub color:     [f32; 4],   // RGBA
    pub fate:      u32,        // 0=Bounded 1=Escaping 2=Looping
}
```

---

## Mathematical background

### Geodesic equations

On a Riemannian surface with metric `g_{ij}` a geodesic `γ(t)` satisfies:

```
d²uⁱ/dt² + Γⁱⱼₖ (duʲ/dt)(duᵏ/dt) = 0
```

where `Γⁱⱼₖ = ½ gⁱˡ (∂ⱼgₗₖ + ∂ₖgₗⱼ − ∂ₗgⱼₖ)` are the Christoffel symbols of the second kind. All fourteen built-in surfaces provide analytic `christoffel()` implementations so that the RK4 integrator never approximates these symbols numerically.

### Curvature comparison

| Surface | Gaussian curvature K | Geodesic character |
|---------|---------------------|-------------------|
| Sphere | K = +1/R² (constant) | Great circles — all geodesics are closed |
| Torus | Mixed (positive outer, negative inner) | Depends on winding ratio: rational = periodic, irrational = dense (ergodic) |
| Saddle / flat | K = 0 | Straight lines in parameter space |
| Catenoid / helicoid | K < 0 (minimal) | Geodesics spiral and diverge |
| Pseudosphere | K = −1 (constant) | Maximal divergence — model of the hyperbolic plane |
| Hyperboloid | K < 0 | Asymptotic geodesics along the rulings |

### Gauss-Bonnet theorem

For any compact surface `Σ` without boundary:

```
∬_Σ K dA = 2π χ(Σ)
```

where `χ` is the Euler characteristic. This connects the local curvature of each built-in surface to its global topology (sphere: χ=2, torus: χ=0, Klein bottle: χ=0, RP²: χ=1).

---

## Architecture

| Module | Responsibility |
|--------|----------------|
| `config` | Load and hot-reload `config.toml`; parse CSS hex colours |
| `error` | Typed error enum covering all subsystems |
| `surface` | `Surface` trait + 14 implementations |
| `geodesic` | RK4 integrator for the geodesic ODE |
| `field` | Dense geodesic field (basin visualization, `FlowArrow` instances) |
| `interactive` | Mouse event handling; geodesic shooting |
| `trail` | Fixed-capacity ring buffer with quadratic alpha fade |
| `renderer` | wgpu render pipelines (surface wireframe + trail lines) |
| `wallpaper` | Win32 borderless window pinned below all app windows |
| `gallery` | Auto-cycle through surfaces in gallery mode with cross-fade |
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

## Animation Export (`src/animation.rs`)

Export a sequence of PNG frames by interpolating one or more parameters over time.

### CLI

```bash
geodesic-wallpaper --animate --frames 60 --fps 30 --out-dir ./frames
```

Frames are written as `frames/frame_0000.png` through `frames/frame_0059.png`.

### Animated parameters

| `AnimationParameter` | Description |
|---------------------|-------------|
| `RotationAngle` | Camera orbit angle (radians) |
| `Scale` | Scene scale factor |
| `ColorHue` | Hue rotation of the color palette (degrees) |
| `WindingNumber` | Symmetry winding number |

### Interpolation modes

| Mode | Formula |
|------|---------|
| `Linear` | `start + (end - start) * t` |
| `Sinusoidal` | `start + (end - start) * 0.5 * (1 - cos(π·t))` |

### API example

```rust
use geodesic_wallpaper::animation::{
    AnimationConfig, AnimationExporter, AnimationParameter, FrameInterpolator, InterpolationMode,
};
use std::path::PathBuf;

let config = AnimationConfig {
    frames: 60, fps: 30, width: 1920, height: 1080,
    output_dir: PathBuf::from("./frames"),
};
let interp = FrameInterpolator::new(
    AnimationParameter::RotationAngle, 0.0, std::f64::consts::TAU, InterpolationMode::Linear,
);
let exporter = AnimationExporter::new(config, vec![interp]);
let stats = exporter.export(|frame_idx, params, path| {
    // render frame to path
    Ok(())
}).unwrap();
println!("{} frames in {}ms", stats.frames_written, stats.duration_ms);
```

---

## Wallpaper Symmetry Groups (`src/symmetry.rs`)

Two high-complexity wallpaper groups — **p4g** and **p6m** — are now implemented.
Each maps any 2D coordinate to a canonical fundamental domain, enabling symmetric
texture and color mapping for geodesic surfaces.

### p4g — Square lattice with glide reflections (8 operations)

```rust
use geodesic_wallpaper::symmetry::{P4g, SymmetryGroup};

let p4g = P4g::new(1.0);
let (u, v) = p4g.to_fundamental_domain(1.3, 2.7);
let orbit = p4g.orbit(0.4, 0.2); // 8 symmetry copies
let color = p4g.color_value(0.7, 0.3); // 0..=1 for palette indexing
```

Symmetry operations: 4 rotations (0°, 90°, 180°, 270°) + 4 diagonal glide reflections.

### p6m — Hexagonal lattice with all reflections (12 operations)

```rust
use geodesic_wallpaper::symmetry::{P6m, SymmetryGroup};

let p6m = P6m::new(1.0);
let (u, v) = p6m.to_fundamental_domain(0.5, 0.8);
let orbit = p6m.orbit(1.0, 0.0); // 12 symmetry copies
```

Symmetry operations: 6 rotations (0°–300° in 60° steps) + 6 reflections.

### Pattern sampling

```rust
use geodesic_wallpaper::symmetry::{P6m, sample_pattern};

let p6m = P6m::new(1.0);
let grid = sample_pattern(&p6m, 256, 256, (-2.0, 2.0), (-2.0, 2.0));
// grid is a 256×256 flat Vec<f32> with values in [0, 1]
```

---

## Color Palette Generator (`src/palette.rs`)

Generate HSL-based color palettes using classical color theory.

### CLI

```bash
geodesic-wallpaper --palette triadic:240 --palette-steps 8
geodesic-wallpaper --palette rainbow --palette-steps 12
geodesic-wallpaper --palette monochromatic:120 --palette-steps 6
```

### Palette types

| Type | Description |
|------|-------------|
| `rainbow` | Evenly spread hues across the full 360° wheel |
| `monochromatic:HUE` | Shades of a single hue (varying lightness) |
| `complementary:HUE` | Two opposing hues (180° apart) |
| `triadic:HUE` | Three equidistant hues (120° apart) |
| `analogous:HUE` | Adjacent hues (±30° from base) |

### API example

```rust
use geodesic_wallpaper::palette::{PaletteGenerator, PaletteType, hsl_to_rgb};

// Generate a triadic palette with 6 colors based at hue 240° (blue)
let palette = PaletteGenerator::generate(PaletteType::Triadic(240.0), 6);
println!("Palette: {}", palette.name);
for hex in palette.to_hex_strings() {
    println!("  {}", hex);
}

// Parse a palette spec from a string (e.g. from CLI)
let p = PaletteGenerator::from_spec("analogous:60", 8).unwrap();

// HSL to RGB conversion
let [r, g, b] = hsl_to_rgb(120.0, 0.8, 0.5); // green
```

---

## License

MIT — see [LICENSE](LICENSE) for details.
