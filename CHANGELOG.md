# Changelog

All notable changes to geodesic-wallpaper are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

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
