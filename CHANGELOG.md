# Changelog

All notable changes to geodesic-wallpaper are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

- No changes pending.

---

## [1.6.0] - 2026-10-03

### Added

- `--gif FILE` (with `--headless`): writes the recorded frames straight to a looping animated GIF, no ffmpeg needed. Frames follow `--record-start` / `--record-every` and play at 30 / every fps.
- `--apng FILE`: the same frames as a looping animated PNG in full colour (`recorder::ApngWriter`, via the `png` crate).
- `--mp4 FILE` and `--ffmpeg PATH`: the same frames as an H.264 MP4, piped to an installed ffmpeg (`recorder::FfmpegWriter`). A missing ffmpeg is a clear error that suggests `--gif`/`--apng`.
- `--completions SHELL`: shell completion scripts via `clap_complete`.
- `palette` feature: `From` conversions between `colorspace::{Rgb, Lab, Oklab}` and the `palette` crate, with a test that checks our Lab and Oklab maths against palette on 4,913 colours.
- `TrailBuffer::append_ordered`: flattens a trail into a reused buffer. Per-frame CPU work (30 geodesics, 300-point trails) went from 110.9 to 28.4 µs on the torus (`cargo bench --bench simulation`, same bench built against 1.5.1); output images are byte-identical.
- Property tests check both expression parsers against the `exmex` crate on random expressions, and that arbitrary text never panics.
- Examples `custom_surface` and `trails_to_gif`; criterion bench `simulation`; GitLab issue templates; `#![warn(missing_docs)]` with every public item documented.
- `recorder::GifWriter`: streams RGBA frames into a GIF (NeuQuant palette per frame, speed 10). `encode_to_gif` now uses it.
- `hot_reload::watch_config`: the debounced `config.toml` watcher, now in the library and tested.
- `export::save_rgba_png`, and `examples/follow_geodesic.rs`.
- `.gitlab-ci.yml`: fmt, clippy, tests (including the `lua` feature) and docs on GitLab's Windows runners.

### Changed

- Config hot reload uses `notify-debouncer-mini`. The old watcher reloaded on the first file event and then ignored events for 200 ms, so a second save inside that window could be lost; now the file is read once after it has been quiet for 250 ms.
- PNG, PPM and BMP export go through the `image` crate. The three hand-written PNG encoders (in `export`, `animation` and `timelapse`) wrote uncompressed (stored) deflate blocks: a 256x256 gradient was 262,289 bytes, now 2,000.
- GIF frame delays are rounded to the nearest centisecond (15 fps was stored as 6 cs, now 7 cs) and never go below 2 cs, which browsers slow down to 10 cs.
- `--tile`, `--fractal`, `--gradient`, `--palette` and `--colorspace` print their output and exit, instead of then also starting the live wallpaper.
- `rust-version` is 1.88, the minimum `image` 0.25.10 declares.

### Fixed

- Expression parsers (user-defined and procedural surfaces) had wrong precedence: `2*x**2` parsed as `(2*x)**2` and `-x^2` as `(-x)^2`. Found by the new property tests against `exmex`; both now follow standard maths precedence.
- The procedural parser rejected scientific notation (`1e-3`); unknown function names in user-defined surfaces silently produced NaN instead of an error; deeply nested input overflowed the stack. All three are now handled (nesting is capped at 64).
- The Enneper surface could not be reached with the `]` / `[` keys.
- `BandSplitter::split` panicked on a spectrum shorter than `fft_size / 2`; a NaN keyframe time panicked in `AnimTrack::add_keyframe`; NaN terrain heights panicked in erosion. All use bounds checks or `total_cmp` now.
- README claimed every surface had exact Christoffel symbols; five use finite differences, and the README now says which.

- The `lua` feature did not compile (non-`Send` Lua state, two type errors). It now builds with `mlua`'s `send` feature and its tests pass.
- `ExportStats::bytes_written` was an estimate for PNG; it is now the real file size for every format.
- Exporting a pixel buffer shorter than `width * height` panicked in the BMP writer; it now returns `ExportError::InvalidDimensions`.
- The timelapse PNG writer wrote a corrupt file when handed a short buffer; it now returns an error.
- `cargo binstall` looked for release zips on GitLab, which has none; it now points at the GitHub releases the install script also uses.

### Removed

- A stray `.claude/worktrees` gitlink from the repository.

---

## [1.5.1] - 2026-09-25

### Fixed

- The renderer failed wgpu validation on its first frame ("Buffer is bound with size 96 where the shader expects 112"), so the wallpaper and `--headless` exited at start. The WGSL uniform padding now matches the Rust struct.
- `background_color` is converted from sRGB, so `#050510` shows as that color instead of a lighter navy.
- Headless renders now use the configured camera and `show_wireframe`, matching the live wallpaper.
- `klein_bottle`, `boy_surface`, `torus_knot`, `pseudosphere` and `trefoil` no longer print an "unrecognised surface" warning.

### Added

- `--record DIR`, `--record-start N`, `--record-every N`, `--width`, `--height`: save real headless frames as numbered PNGs for GIFs and videos, with no window.
- `config.toml` and `presets/` are found next to the exe when they are not in the current folder, so an installed copy works from any terminal.
- `install.ps1` one-line installer, Scoop manifest, `cargo binstall` metadata, winget manifests in `packaging/winget/`.
- `--help` examples; `NO_COLOR` respected; quieter default logging.
- README with a GIF, install table and results; banner and social preview.

---

## [1.5.0] - 2026-09-25

### Added

- Release workflow builds a Windows x86_64 zip (exe, README, LICENSE,
  `config.toml`, `presets/`) plus `SHA256SUMS.txt` on every `v*` tag.
- `--version` flag.

### Fixed

- CI is green again: `cargo fmt`, a crate-level lint allow list for the
  library-only generative-art modules, and the rustdoc link warnings.
- Arithmetic overflow panics in the Perlin/fBm hash helpers
  (`generative_art`, `texture_synthesizer`) and a compile-time overflow in
  `quilts`.
- `stereographic::plane_to_sphere` returned the mirrored z coordinate.
- Fractal renderer now samples pixel centres.

---

## [1.2.0] - 2026-03-17

### Added

- Additional unit tests for mathematical correctness:
  - `Geodesic`: RK4 fourth-order convergence test (error ratio check at two
    step sizes), single-step geodesic-equation constraint test (metric speed
    change bounded to < 1 % for small `dt`).
  - `Torus`: unit-normal length grid test, torsion-free Christoffel symmetry
    over a 6×6 parameter grid.
  - `Sphere`: unit-normal length grid test, metric positive-definiteness grid
    test, torsion-free Christoffel symmetry grid test.
  - `Saddle`: unit-normal length grid test, metric positive-definiteness grid
    test, Christoffel non-zero and symmetric at an off-origin point.
- `.github/workflows/ci.yml`: new `doc` job that runs `cargo doc --no-deps`
  with `RUSTDOCFLAGS=-D warnings`, ensuring all public items are documented.
  A comment notes that full rendering tests require a display and are
  intentionally skipped on headless CI runners.

### Changed

- Version bumped from `1.1.0` to `1.2.0`.

---

## [1.1.0] - 2026-03-17

### Added

- Comprehensive doc comments on every public type, field, and function across
  all modules (`config`, `error`, `geodesic`, `trail`, `surface/*`,
  `renderer/camera`).
- Unit tests for all pure math and geometry functions:
  - `TrailBuffer`: ring-wrap ordering, fade-alpha monotonicity, capacity
    clamping, clear behaviour.
  - `Camera`: finite view-projection matrix, orbit angle accumulation,
    matrix change on orbit.
  - `Torus`: metric positive-definiteness at all sample points, Christoffel
    symmetry, outer/inner equator values, great-circle geodesic drift bound.
  - `Sphere`: radius scaling, metric symmetry, Christoffel at pole/equator,
    wrap behaviour, radial normal, mesh vertex/index counts.
  - `Saddle`: origin embedding, metric identity at origin, Christoffel
    symmetry and zero at origin, wrap clamping, unit normal, mesh counts.
  - `Geodesic`: metric-speed conservation on torus, die-at-max-age,
    sphere periodicity, zero-velocity NaN guard.
- `TrailBuffer::new` now clamps capacity to a minimum of 1 to prevent
  divide-by-zero in `ordered_vertices` when callers pass `0`.
- `.github/workflows/ci.yml`: four-job CI pipeline (fmt, clippy, test, build)
  targeting `x86_64-pc-windows-msvc`, uploading the release binary as a
  workflow artifact on every push to master.
- `Cargo.toml`: added `rust-version = "1.75"`, `homepage`, `readme`, and
  `exclude` fields.

### Changed

- Version bumped from `1.0.0` to `1.1.0`.
- README rewritten: CI badge, feature list, architecture overview, quickstart,
  full configuration reference, prerequisites section, and tech-stack table.
- CI workflow extended from a single job to four separate jobs (`fmt`,
  `clippy`, `test`, `build`) so failures are reported per-stage.

### Fixed

- `TrailBuffer::ordered_vertices` ring-wrap path was correct but untested;
  regression tests added and confirmed passing.
- Sphere Christoffel guard `if sv.abs() > 1e-6` prevents divide-by-zero at the
  poles; covered by a regression test.

---

## [1.0.0] - 2025-01-01

### Added

- Initial public release.
- Real-time geodesic flow wallpaper for Windows 10 and 11.
- Three surface types: torus, sphere, and hyperbolic paraboloid (saddle).
- RK4 integration of the geodesic ODE with analytic Christoffel symbols.
- Velocity renormalisation after each RK4 step to preserve metric speed.
- Ring-buffer trail system with quadratic alpha fade.
- Hot-reload of `config.toml` via `notify` filesystem watcher.
- wgpu-based renderer (DX12 / Vulkan) with surface wireframe and trail pipelines.
- Win32 WorkerW trick to render behind desktop icons.
- Slowly orbiting perspective camera.
- GitHub Actions release workflow that builds and uploads a Windows `.exe` zip
  on every `v*` tag push.
