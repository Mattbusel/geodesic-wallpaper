# Contributing to geodesic-wallpaper

Thank you for your interest in contributing! Please read this guide before
opening a pull request.

---

## Prerequisites

- **Rust stable** (1.75 or later) — install via [rustup](https://rustup.rs/)
- **Windows 10 or 11** — the wallpaper window and wgpu surface depend on Win32
- **wgpu-compatible GPU** — DirectX 12, Vulkan, or Metal support required
- **Git** for version control

---

## Building

```powershell
git clone https://github.com/Mattbusel/geodesic-wallpaper.git
cd geodesic-wallpaper
cargo build --release
```

The release binary lands at `target\release\geodesic-wallpaper.exe`. Copy
`config.toml` from the repo root to the same directory before running.

---

## Running tests

No GPU is required for the test suite; all tests exercise the pure-math and
ring-buffer layers:

```powershell
cargo test --lib
```

Run a specific module:

```powershell
cargo test --lib geodesic
cargo test --lib surface
```

---

## Testing config hot-reload

1. Run the binary: `.\target\release\geodesic-wallpaper.exe`
2. Edit `config.toml` while the application is running (e.g. change `surface`
   from `"torus"` to `"sphere"`).
3. Save the file. The application should reload within ~1 second without
   restarting. Verify the surface changes on screen.

---

## Code style

- **Formatting**: run `cargo fmt --all` before committing.
- **Lints**: the project enforces `-D warnings` via Clippy. Run:
  ```powershell
  cargo clippy --all-targets -- -D warnings
  ```
- **Doc comments**: every public item (function, struct, enum, trait, field)
  must have a `///` doc comment.
- **No unsafe outside `wallpaper.rs`**: all Win32 unsafe code is isolated in
  `src/wallpaper.rs`. New unsafe blocks require a `// SAFETY:` comment.
- **No raw `new`/`delete`**: use smart pointers and RAII.

---

## Opening a pull request

1. Fork the repository and create a feature branch:
   ```powershell
   git checkout -b my-feature
   ```
2. Ensure `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test --lib`
   all pass locally.
3. Open a pull request against `master` with a clear description of the change.

CI enforces formatting, Clippy, the test suite, rustdoc with `-D warnings`,
and a release build before merging.
