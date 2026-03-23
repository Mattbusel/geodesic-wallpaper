//! Library façade exposing all public modules for integration tests and `cargo doc`.

pub mod config;
pub mod error;
pub mod events;
pub mod gallery;
pub mod geodesic;
#[cfg(feature = "lua")]
pub mod lua_surface;
pub mod parameter_tuner;
pub mod recorder;
pub mod renderer;
pub mod surface;
pub mod trail;
pub mod tray;
pub mod wallpaper;
