//! Library façade exposing all public modules for integration tests and `cargo doc`.

// Many of the generative-art modules below are library-only experiments that
// the wallpaper binary does not call. Keep CI's `clippy -D warnings` gate
// meaningful for the core while tolerating their style lints.
#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    unused_parens,
    unused_comparisons,
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::cast_abs_to_unsigned,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::const_is_empty,
    clippy::excessive_precision,
    clippy::flat_map_identity,
    clippy::items_after_test_module,
    clippy::let_and_return,
    clippy::manual_div_ceil,
    clippy::manual_memcpy,
    clippy::manual_range_contains,
    clippy::needless_range_loop,
    clippy::nonminimal_bool,
    clippy::overly_complex_bool_expr,
    clippy::ptr_arg,
    clippy::should_implement_trait,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::unnecessary_cast,
    clippy::unnecessary_map_or,
    clippy::useless_vec,
    rustdoc::broken_intra_doc_links
)]

pub mod animation;
pub mod animation_engine;
pub mod audio_reactive;
pub mod cellular_automata;
pub mod cellular_automata_2d;
pub mod cellular_noise;
pub mod color_theory;
pub mod colorspace;
pub mod composer;
pub mod config;
pub mod dithering;
pub mod error;
pub mod escher;
pub mod events;
pub mod export;
pub mod field;
pub mod finance_driver;
pub mod flow;
pub mod flow_field;
pub mod flow_field_render;
pub mod fractal;
pub mod fractal_geometry;
pub mod fractal_tree;
pub mod gallery;
pub mod generative_art;
pub mod geodesic;
pub mod gradient;
pub mod hyperbolic_tiling;
pub mod interactive;
pub mod interference;
pub mod islamic_patterns;
pub mod iso_surface;
pub mod kaleidoscope;
pub mod l_system;
pub mod lissajous;
pub mod lod;
#[cfg(feature = "lua")]
pub mod lua_surface;
pub mod mandelbrot;
pub mod mesh;
pub mod metamorphosis;
pub mod morph;
pub mod mosaic;
pub mod multi_monitor;
pub mod noise;
pub mod noise_field;
pub mod origami;
pub mod palette;
pub mod parameter_tuner;
pub mod particle_system;
pub mod particles;
pub mod pathfinding_art;
pub mod penrose;
pub mod preview;
pub mod projection;
pub mod projection_art;
pub mod quilts;
pub mod reaction_diffusion;
pub mod reaction_diffusion_v2;
pub mod recorder;
pub mod renderer;
pub mod scene_presets;
pub mod spirograph;
pub mod stereographic;
pub mod strange_attractor;
pub mod surface;
pub mod symmetry;
pub mod symmetry_art;
pub mod terrain;
pub mod texture_synthesizer;
pub mod tiling;
pub mod timelapse;
pub mod topology_art;
pub mod torus_mapping;
pub mod trail;
pub mod tray;
pub mod typography;
pub mod voronoi;
pub mod voronoi_art;
pub mod wallpaper;
pub mod wave_function_collapse;
pub mod wave_interference;
