# Geodesic Flow: live desktop wallpaper

[![CI](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/ci.yml/badge.svg)](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/ci.yml)
[![Release](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/release.yml/badge.svg)](https://github.com/Mattbusel/geodesic-wallpaper/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.75+](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

**An animated Windows wallpaper that draws geodesics, the "straightest possible" paths, flowing across curved surfaces in real time. Rust, wgpu, and an RK4 integrator on exact Christoffel symbols.**

A geodesic on a torus, a saddle or a Klein bottle behaves very differently from a straight line on a plane: it spirals, precesses, closes up or fills the surface densely depending on the curvature. This project turns that into ambient art. Dozens of colored trails wind across one of fourteen analytic surfaces while the camera slowly orbits, and the window sits behind your desktop icons so it behaves like a normal wallpaper.

## Features

- **Fourteen surfaces** with analytic metrics and Christoffel symbols: torus, sphere, saddle, catenoid, helicoid, hyperboloid, hyperbolic paraboloid, ellipsoid, Enneper, Klein bottle, Boy's surface, torus knot, pseudosphere and trefoil tube.
- **RK4 integration** of the geodesic equation in parameter space, with trails that fade by a configurable power law.
- **Real wallpaper behavior**: a borderless Win32 window pinned below all application windows, so desktop icons stay usable. Choose the primary monitor, a specific monitor, or span all of them.
- **Hot-reloaded `config.toml`**: edit and save, and the running wallpaper picks up the change.
- **Presets** (`--preset cosmic`, `fire`, `matrix`, `neon`, `ocean`) loaded from `presets/`.
- **Headless render** to PNG for thumbnails or CI (`--headless`).
- **System tray icon** and keyboard controls.

## Quick start

### Download (Windows)

1. Open the [latest release](https://github.com/Mattbusel/geodesic-wallpaper/releases/latest) and download `geodesic-wallpaper-vX.Y.Z-x86_64-pc-windows-msvc.zip`.
2. Extract it and run `geodesic-wallpaper.exe`. The zip also holds a sample `config.toml` and the `presets/` folder.
3. The exe is unsigned, so Windows SmartScreen may say "unknown publisher": click **More info**, then **Run anyway**. You can check the download against `SHA256SUMS.txt` on the release page.

Requires Windows 10 or 11 and a GPU with DirectX 12 or Vulkan. There are no macOS or Linux builds: the app is a Win32 desktop wallpaper.

### Install with Cargo (Windows)

```powershell
cargo install geodesic-wallpaper
```

### Build from source

```powershell
git clone https://github.com/Mattbusel/geodesic-wallpaper.git
cd geodesic-wallpaper
cargo run --release                      # uses ./config.toml
cargo run --release -- --preset cosmic   # presets/cosmic.toml over config.toml
cargo run --release -- --headless --frames 300 --output shot.png
cargo test --lib                         # no GPU needed
```

`--preset` reads `presets/<name>.toml` relative to the working directory, so run from the repo root or copy the `presets/` folder next to the exe.

## Controls

| Key | Action |
| --- | --- |
| `]` / `[` | Next / previous surface |
| `+` / `-` | Speed up / slow down |
| `R` | Reset all geodesics |
| `F` | Toggle FPS overlay |
| `Space` | Pause |
| `P` | Save a screenshot |

Right-click the tray icon for the tray menu.

## Configuration

All fields are optional. Missing fields revert to the defaults shown.

Place `config.toml` in the same directory as the executable. The file is hot-reloaded automatically whenever it changes on disk, no restart required.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `surface` | string | `"torus"` | Surface to render. See [Supported surfaces](#supported-surfaces). |
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
| `monitor` | string | `"primary"` | `"primary"`, `"all"` (span every monitor) or a monitor index such as `"1"`. |

Minimal example:

```toml
surface          = "torus"
num_geodesics    = 30
trail_length     = 300
rotation_speed   = 0.001047
background_color = "#050510"
color_palette    = ["#4488FF", "#88DDFF", "#FFD700", "#88FF88", "#FF88CC"]
```

## Command-line flags

| Flag | Effect |
| --- | --- |
| `--preset NAME` | Merge `presets/NAME.toml` over `config.toml` |
| `--headless [--frames N] [--output FILE] [--output-format png\|ppm\|bmp\|svg]` | Simulate N frames without a window and save the last one |
| `--preview` | Print an ASCII block preview of the pattern and exit |
| `--animate [--frames N] [--fps N] [--out-dir DIR]` | Exercise the frame exporter; currently writes gradient test frames, not wallpaper renders |
| `--palette TYPE[:HUE] [--palette-steps N]` | Print a generated palette (rainbow, monochromatic, complementary, triadic, analogous) |
| `--gradient`, `--colorspace`, `--fractal`, `--tile` | Print diagnostics from the gradient, color space, fractal and tiling modules |

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
| **Pseudosphere** | `"pseudosphere"` | **Constant negative K = −1** | Tractricoid; geodesics diverge exponentially; the hyperbolic plane's classic model surface |
| **Trefoil tube** | `"trefoil"` | Positive (tube) | Circular cross-section swept around the trefoil knot curve; geodesics precess across all three lobes |

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
| Sphere | K = +1/R² (constant) | Great circles: all geodesics are closed |
| Torus | Mixed (positive outer, negative inner) | Depends on winding ratio: rational = periodic, irrational = dense (ergodic) |
| Saddle / flat | K = 0 | Straight lines in parameter space |
| Catenoid / helicoid | K < 0 (minimal) | Geodesics spiral and diverge |
| Pseudosphere | K = −1 (constant) | Maximal divergence: model of the hyperbolic plane |
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
| --- | --- |
| `surface/` | `Surface` trait and the fourteen implementations, plus procedural and user-defined surfaces |
| `geodesic` | RK4 integrator for the geodesic ODE |
| `trail` | Fixed-capacity ring buffer with power-law alpha fade |
| `renderer/` | wgpu pipelines and WGSL shaders for the surface wireframe and trails, camera |
| `wallpaper` | Win32 window pinned below app windows, keyboard input, monitor enumeration |
| `config` | `config.toml` loading, profiles and hot reload |
| `tray` | System tray icon |
| `main` | CLI, message loop, render loop, headless path |

The crate is also a library (`geodesic_wallpaper`) with a large set of additional modules that are unit tested but **not yet wired into the wallpaper binary**: gallery mode, a live parameter tuner, phase portrait recording, mouse-driven geodesic shooting, a geodesic field (basin) view, per-monitor surface assignment, a financial OHLCV data driver, Lua-scripted surfaces (`--features lua`), scene presets, and generative-art modules (wallpaper symmetry groups, Penrose and Escher tilings, reaction-diffusion, L-systems, strange attractors, Voronoi, fractals, color spaces and export formats). Their `config.toml` keys (for example `gallery_mode`) parse but currently have no effect on the running wallpaper.

## Status

Working Windows wallpaper, version 1.5.0. `cargo test --lib` is the quickest local check. Contributions: see [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT, see [LICENSE](LICENSE).
