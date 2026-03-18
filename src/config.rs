//! Runtime configuration loaded from `config.toml` with hot-reload support.
//!
//! All fields have serde defaults so the application starts with sensible
//! values even when the config file is absent or partially specified.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Runtime configuration for the geodesic wallpaper.
///
/// Loaded from `config.toml` on startup and re-loaded whenever the file
/// changes on disk (hot-reload). Missing fields fall back to their defaults.
///
/// # Examples
///
/// ```
/// use geodesic_wallpaper::config::Config;
///
/// let cfg = Config::default();
/// assert_eq!(cfg.surface, "torus");
/// assert_eq!(cfg.num_geodesics, 30);
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct Config {
    /// Surface to render: `"torus"`, `"sphere"`, or `"saddle"`.
    ///
    /// Any unrecognised value falls back to `"torus"`.
    #[serde(default = "default_surface")]
    pub surface: String,

    /// Number of simultaneous geodesic curves.
    ///
    /// Default: `30`.
    #[serde(default = "default_num_geodesics")]
    pub num_geodesics: usize,

    /// Number of frames a trail persists before fading out.
    ///
    /// Default: `300`.
    #[serde(default = "default_trail_length")]
    pub trail_length: usize,

    /// Camera orbit speed in radians per second.
    ///
    /// Default: `0.001047` (approximately one revolution every 100 minutes).
    #[serde(default = "default_rotation_speed")]
    pub rotation_speed: f32,

    /// Trail colour palette as CSS hex strings (e.g. `"#4488FF"`).
    ///
    /// Geodesics cycle through this list. At least one colour is required;
    /// the default palette contains five entries.
    #[serde(default = "default_color_palette")]
    pub color_palette: Vec<String>,

    /// Torus major radius: distance from the torus center to the tube center.
    ///
    /// Default: `2.0`.
    #[serde(default = "default_torus_r_big")]
    #[allow(non_snake_case)]
    pub torus_R: f32,

    /// Torus minor radius: tube radius.
    ///
    /// Default: `0.7`.
    #[serde(default = "default_torus_r_small")]
    pub torus_r: f32,

    /// RK4 integration timestep per frame in seconds.
    ///
    /// The value `0.04` overshoots torus geodesics at 30 fps; the default
    /// `0.016` keeps trajectories numerically stable.
    ///
    /// Default: `0.016`.
    #[serde(default = "default_time_step")]
    pub time_step: f32,
}

fn default_surface() -> String { "torus".into() }
fn default_num_geodesics() -> usize { 30 }
fn default_trail_length() -> usize { 300 }
fn default_rotation_speed() -> f32 { 0.001047 }
fn default_color_palette() -> Vec<String> {
    vec![
        "#4488FF".into(),
        "#88DDFF".into(),
        "#FFD700".into(),
        "#88FF88".into(),
        "#FF88CC".into(),
    ]
}
fn default_torus_r_big() -> f32 { 2.0 }
fn default_torus_r_small() -> f32 { 0.7 }
fn default_time_step() -> f32 { 0.016 }

impl Default for Config {
    fn default() -> Self {
        Config {
            surface: default_surface(),
            num_geodesics: default_num_geodesics(),
            trail_length: default_trail_length(),
            rotation_speed: default_rotation_speed(),
            color_palette: default_color_palette(),
            torus_R: default_torus_r_big(),
            torus_r: default_torus_r_small(),
            time_step: default_time_step(),
        }
    }
}

impl Config {
    /// Load a [`Config`] from a TOML file at `path`.
    ///
    /// If the file cannot be read or the TOML cannot be parsed, a warning is
    /// logged and the default config is returned. This function never panics.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use geodesic_wallpaper::config::Config;
    /// use std::path::Path;
    ///
    /// let cfg = Config::load(Path::new("config.toml"));
    /// println!("Surface: {}", cfg.surface);
    /// ```
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
                tracing::warn!("Config parse error: {e}, using defaults");
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    /// Parse a CSS hex colour string into a linear `[r, g, b, 1.0]` array.
    ///
    /// Accepts strings with or without a leading `#`. Individual channel
    /// parse failures fall back to `128` (≈ 0.502).
    ///
    /// # Examples
    ///
    /// ```
    /// use geodesic_wallpaper::config::Config;
    ///
    /// let color = Config::parse_color("#FF8800");
    /// assert!((color[0] - 1.0).abs() < 0.01);
    /// assert_eq!(color[3], 1.0);
    /// ```
    pub fn parse_color(hex: &str) -> [f32; 4] {
        let h = hex.trim_start_matches('#');
        let r = u8::from_str_radix(h.get(0..2).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
        let g = u8::from_str_radix(h.get(2..4).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
        let b = u8::from_str_radix(h.get(4..6).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
        [r, g, b, 1.0]
    }
}

/// Thread-safe handle to a [`Config`] that can be updated from a watcher thread.
pub type SharedConfig = Arc<RwLock<Config>>;

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_values_are_correct() {
        let cfg = Config::default();
        assert_eq!(cfg.surface, "torus");
        assert_eq!(cfg.num_geodesics, 30);
        assert_eq!(cfg.trail_length, 300);
        assert!((cfg.rotation_speed - 0.001047).abs() < 1e-6);
        assert!((cfg.torus_R - 2.0).abs() < 1e-6);
        assert!((cfg.torus_r - 0.7).abs() < 1e-6);
        assert!((cfg.time_step - 0.016).abs() < 1e-6);
        assert_eq!(cfg.color_palette.len(), 5);
    }

    #[test]
    fn toml_parse_full_config() {
        let toml = r##"
surface = "sphere"
num_geodesics = 10
trail_length = 100
rotation_speed = 0.005
color_palette = ["#FF0000"]
torus_R = 3.0
torus_r = 1.0
time_step = 0.008
"##;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml.as_bytes()).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "sphere");
        assert_eq!(cfg.num_geodesics, 10);
        assert_eq!(cfg.trail_length, 100);
        assert!((cfg.rotation_speed - 0.005).abs() < 1e-6);
        assert!((cfg.torus_R - 3.0).abs() < 1e-6);
        assert!((cfg.torus_r - 1.0).abs() < 1e-6);
        assert!((cfg.time_step - 0.008).abs() < 1e-6);
        assert_eq!(cfg.color_palette, vec!["#FF0000"]);
    }

    #[test]
    fn partial_config_falls_back_to_defaults() {
        let toml = r#"surface = "saddle""#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml.as_bytes()).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "saddle");
        // Fields not in the file keep their defaults.
        assert_eq!(cfg.num_geodesics, 30);
        assert_eq!(cfg.trail_length, 300);
    }

    #[test]
    fn invalid_toml_returns_defaults() {
        let toml = b"this is not valid toml :::";
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "torus");
        assert_eq!(cfg.num_geodesics, 30);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let cfg = Config::load(std::path::Path::new("/nonexistent/path/config.toml"));
        assert_eq!(cfg.surface, "torus");
    }

    #[test]
    fn parse_color_full_hex() {
        let c = Config::parse_color("#FF8800");
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 0.533).abs() < 0.01);
        assert!((c[2] - 0.0).abs() < 0.01);
        assert_eq!(c[3], 1.0);
    }

    #[test]
    fn parse_color_without_hash() {
        let c_hash = Config::parse_color("#4488FF");
        let c_no_hash = Config::parse_color("4488FF");
        assert_eq!(c_hash, c_no_hash);
    }

    #[test]
    fn parse_color_invalid_falls_back() {
        // Short string: individual channels fall back to 128/255.
        let c = Config::parse_color("#ZZZZZZ");
        assert!((c[0] - 128.0 / 255.0).abs() < 0.01);
        assert_eq!(c[3], 1.0);
    }

    /// The default config must pass basic sanity checks: radii positive, time
    /// step positive, colour palette non-empty.
    #[test]
    fn test_default_config_valid() {
        let cfg = Config::default();
        assert!(cfg.torus_R > 0.0, "torus_R must be positive");
        assert!(cfg.torus_r > 0.0, "torus_r must be positive");
        assert!(cfg.time_step > 0.0, "time_step must be positive");
        assert!(cfg.rotation_speed >= 0.0, "rotation_speed must be non-negative");
        assert!(!cfg.color_palette.is_empty(), "color_palette must not be empty");
        assert!(cfg.trail_length > 0, "trail_length must be > 0");
        // Surface name must be one of the known values.
        assert!(["torus", "sphere", "saddle"].contains(&cfg.surface.as_str()),
            "unexpected default surface: {}", cfg.surface);
    }

    /// Serialise the default config to TOML and deserialise it again; all
    /// fields must survive the round-trip unchanged.
    #[test]
    fn test_config_round_trip() {
        let original = Config::default();
        let toml_str = toml::to_string(&original)
            .expect("serialization failed");
        let restored: Config = toml::from_str(&toml_str)
            .expect("deserialization failed");

        assert_eq!(original.surface, restored.surface);
        assert_eq!(original.num_geodesics, restored.num_geodesics);
        assert_eq!(original.trail_length, restored.trail_length);
        assert!((original.rotation_speed - restored.rotation_speed).abs() < 1e-9);
        assert_eq!(original.color_palette, restored.color_palette);
        assert!((original.torus_R - restored.torus_R).abs() < 1e-9);
        assert!((original.torus_r - restored.torus_r).abs() < 1e-9);
        assert!((original.time_step - restored.time_step).abs() < 1e-9);
    }

    /// The default number of geodesics must be strictly positive.
    #[test]
    fn test_config_geodesic_count_nonzero() {
        let cfg = Config::default();
        assert!(cfg.num_geodesics > 0,
            "num_geodesics must be > 0, got {}", cfg.num_geodesics);
    }
}
