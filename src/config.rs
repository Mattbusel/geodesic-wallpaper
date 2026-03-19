//! Runtime configuration loaded from `config.toml` with hot-reload support.
//!
//! All fields have serde defaults so the application starts with sensible
//! values even when the config file is absent or partially specified.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Surface to render: `"torus"`, `"sphere"`, `"saddle"`, `"catenoid"`,
    /// `"helicoid"`, or `"hyperboloid"`.
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
    /// Default: `0.016`.
    #[serde(default = "default_time_step")]
    pub time_step: f32,

    /// Optional RNG seed for reproducible geodesic spawning.
    ///
    /// When `None` (the default) entropy is used.
    #[serde(default)]
    pub seed: Option<u64>,

    /// Background clear colour as a CSS hex string.
    ///
    /// Default: `"#050510"`.
    #[serde(default = "default_background_color")]
    pub background_color: String,

    /// Trail rendering mode: `"line"`, `"ribbon"`, or `"glow"`.
    ///
    /// Default: `"line"`.
    #[serde(default = "default_trail_mode")]
    pub trail_mode: String,

    /// Speed at which trail colours cycle through the hue wheel (radians/s).
    ///
    /// Default: `0.0` (no cycling).
    #[serde(default = "default_color_cycle_speed")]
    pub color_cycle_speed: f32,

    /// Optional gradient stop colours as CSS hex strings.
    ///
    /// Default: empty (no gradient override).
    #[serde(default)]
    pub gradient_stops: Vec<String>,

    /// Gradient mode: `"none"`, `"linear"`, etc.
    ///
    /// Default: `"none"`.
    #[serde(default = "default_gradient_mode")]
    pub gradient_mode: String,

    /// Name of the active profile to overlay on top of this config.
    ///
    /// When `None` (the default) no profile is applied.
    #[serde(default)]
    pub active_profile: Option<String>,

    /// Named configuration profiles that can override individual fields.
    #[serde(default)]
    pub profiles: HashMap<String, PartialConfig>,

    /// How often (in seconds) to automatically cycle through `presets_order`.
    ///
    /// `None` disables automatic preset cycling.
    #[serde(default)]
    pub preset_cycle_secs: Option<f32>,

    /// Ordered list of preset names to cycle through.
    #[serde(default)]
    pub presets_order: Vec<String>,

    /// Scale factor for the catenoid surface.
    ///
    /// Default: `1.0`.
    #[serde(default = "default_catenoid_c")]
    pub catenoid_c: f32,

    /// Scale factor for the helicoid surface.
    ///
    /// Default: `1.0`.
    #[serde(default = "default_helicoid_c")]
    pub helicoid_c: f32,

    /// Semi-axis `a` for the hyperboloid surface.
    ///
    /// Default: `1.0`.
    #[serde(default = "default_hyperboloid_a")]
    pub hyperboloid_a: f32,

    /// Semi-axis `b` for the hyperboloid surface.
    ///
    /// Default: `1.0`.
    #[serde(default = "default_hyperboloid_b")]
    pub hyperboloid_b: f32,

    /// Directional light direction vector `[x, y, z]`.
    ///
    /// Default: `[1.0, 1.0, 1.0]`.
    #[serde(default = "default_light_dir")]
    pub light_dir: [f32; 3],

    /// Whether hue-cycling of trail colours is enabled.
    ///
    /// Default: `false`.
    #[serde(default = "default_color_cycle_enabled")]
    pub color_cycle_enabled: bool,
}

// ─── Default helpers ──────────────────────────────────────────────────────────

fn default_surface() -> String {
    "torus".into()
}
fn default_num_geodesics() -> usize {
    30
}
fn default_trail_length() -> usize {
    300
}
fn default_rotation_speed() -> f32 {
    0.001047
}
fn default_color_palette() -> Vec<String> {
    vec![
        "#4488FF".into(),
        "#88DDFF".into(),
        "#FFD700".into(),
        "#88FF88".into(),
        "#FF88CC".into(),
    ]
}
fn default_torus_r_big() -> f32 {
    2.0
}
fn default_torus_r_small() -> f32 {
    0.7
}
fn default_time_step() -> f32 {
    0.016
}
fn default_background_color() -> String {
    "#050510".into()
}
fn default_trail_mode() -> String {
    "line".into()
}
fn default_color_cycle_speed() -> f32 {
    0.0
}
fn default_gradient_mode() -> String {
    "none".into()
}
fn default_catenoid_c() -> f32 {
    1.0
}
fn default_helicoid_c() -> f32 {
    1.0
}
fn default_hyperboloid_a() -> f32 {
    1.0
}
fn default_hyperboloid_b() -> f32 {
    1.0
}
fn default_light_dir() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn default_color_cycle_enabled() -> bool {
    false
}

// ─── PartialConfig ────────────────────────────────────────────────────────────

/// A mirror of [`Config`] where every field is optional.
///
/// Used to represent named configuration profiles; only the fields that are
/// explicitly set in a profile will override the base config when
/// [`Config::resolve_profile`] is called.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct PartialConfig {
    pub surface: Option<String>,
    pub num_geodesics: Option<usize>,
    pub trail_length: Option<usize>,
    pub rotation_speed: Option<f32>,
    pub color_palette: Option<Vec<String>>,
    #[allow(non_snake_case)]
    pub torus_R: Option<f32>,
    pub torus_r: Option<f32>,
    pub time_step: Option<f32>,
    pub seed: Option<u64>,
    pub background_color: Option<String>,
    pub trail_mode: Option<String>,
    pub color_cycle_speed: Option<f32>,
    pub gradient_stops: Option<Vec<String>>,
    pub gradient_mode: Option<String>,
    pub preset_cycle_secs: Option<f32>,
    pub presets_order: Option<Vec<String>>,
    pub catenoid_c: Option<f32>,
    pub helicoid_c: Option<f32>,
    pub hyperboloid_a: Option<f32>,
    pub hyperboloid_b: Option<f32>,
    pub light_dir: Option<[f32; 3]>,
    pub color_cycle_enabled: Option<bool>,
}

// ─── impl Default / Config ────────────────────────────────────────────────────

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
            seed: None,
            background_color: default_background_color(),
            trail_mode: default_trail_mode(),
            color_cycle_speed: default_color_cycle_speed(),
            gradient_stops: Vec::new(),
            gradient_mode: default_gradient_mode(),
            active_profile: None,
            profiles: HashMap::new(),
            preset_cycle_secs: None,
            presets_order: Vec::new(),
            catenoid_c: default_catenoid_c(),
            helicoid_c: default_helicoid_c(),
            hyperboloid_a: default_hyperboloid_a(),
            hyperboloid_b: default_hyperboloid_b(),
            light_dir: default_light_dir(),
            color_cycle_enabled: default_color_cycle_enabled(),
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

    /// Compute the effective colour palette, applying gradient interpolation if configured.
    ///
    /// - `"none"` or empty stops: returns `color_palette` parsed as RGBA.
    /// - `"linear"`: linearly interpolates RGB between `gradient_stops`.
    /// - `"hsv"`: interpolates in HSV space between `gradient_stops`.
    pub fn effective_colors(&self) -> Vec<[f32; 4]> {
        if self.gradient_mode == "none" || self.gradient_stops.is_empty() {
            return self.color_palette.iter().map(|s| Self::parse_color(s)).collect();
        }

        let stops: Vec<[f32; 4]> = self.gradient_stops.iter().map(|s| Self::parse_color(s)).collect();
        let n = self.num_geodesics.max(1);

        match self.gradient_mode.as_str() {
            "linear" => (0..n)
                .map(|i| {
                    let t = i as f32 / (n - 1).max(1) as f32;
                    Self::lerp_color_linear(&stops, t)
                })
                .collect(),
            "hsv" => (0..n)
                .map(|i| {
                    let t = i as f32 / (n - 1).max(1) as f32;
                    Self::lerp_color_hsv(&stops, t)
                })
                .collect(),
            _ => self.color_palette.iter().map(|s| Self::parse_color(s)).collect(),
        }
    }

    fn lerp_color_linear(stops: &[[f32; 4]], t: f32) -> [f32; 4] {
        if stops.len() == 1 {
            return stops[0];
        }
        let seg = t * (stops.len() - 1) as f32;
        let idx = (seg as usize).min(stops.len() - 2);
        let frac = seg - idx as f32;
        let a = stops[idx];
        let b = stops[idx + 1];
        [
            a[0] + (b[0] - a[0]) * frac,
            a[1] + (b[1] - a[1]) * frac,
            a[2] + (b[2] - a[2]) * frac,
            1.0,
        ]
    }

    fn lerp_color_hsv(stops: &[[f32; 4]], t: f32) -> [f32; 4] {
        if stops.len() == 1 {
            return stops[0];
        }
        let seg = t * (stops.len() - 1) as f32;
        let idx = (seg as usize).min(stops.len() - 2);
        let frac = seg - idx as f32;
        let ha = Self::rgb_to_hsv(stops[idx]);
        let hb = Self::rgb_to_hsv(stops[idx + 1]);
        // Interpolate hue along shortest arc
        let mut dh = hb[0] - ha[0];
        if dh > 0.5 { dh -= 1.0; }
        if dh < -0.5 { dh += 1.0; }
        let h = (ha[0] + dh * frac).rem_euclid(1.0);
        let s = ha[1] + (hb[1] - ha[1]) * frac;
        let v = ha[2] + (hb[2] - ha[2]) * frac;
        Self::hsv_to_rgb([h, s, v])
    }

    fn rgb_to_hsv(c: [f32; 4]) -> [f32; 3] {
        let (r, g, b) = (c[0], c[1], c[2]);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let v = max;
        let s = if max < 1e-6 { 0.0 } else { delta / max };
        let h = if delta < 1e-6 {
            0.0
        } else if max == r {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if max == g {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };
        [h, s, v]
    }

    fn hsv_to_rgb(hsv: [f32; 3]) -> [f32; 4] {
        let (h, s, v) = (hsv[0], hsv[1], hsv[2]);
        let i = (h * 6.0).floor() as i32;
        let f = h * 6.0 - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - f * s);
        let t = v * (1.0 - (1.0 - f) * s);
        let (r, g, b) = match i % 6 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        [r, g, b, 1.0]
    }

    /// Apply the active profile (if any) on top of `self`, returning a merged
    /// [`Config`].
    ///
    /// If `active_profile` is `None` or names a profile that does not exist in
    /// `profiles`, `self` is returned unchanged (cloned).
    pub fn resolve_profile(&self) -> Config {
        let profile = match &self.active_profile {
            Some(name) => match self.profiles.get(name) {
                Some(p) => p.clone(),
                None => return self.clone(),
            },
            None => return self.clone(),
        };

        let mut out = self.clone();
        if let Some(v) = profile.surface { out.surface = v; }
        if let Some(v) = profile.num_geodesics { out.num_geodesics = v; }
        if let Some(v) = profile.trail_length { out.trail_length = v; }
        if let Some(v) = profile.rotation_speed { out.rotation_speed = v; }
        if let Some(v) = profile.color_palette { out.color_palette = v; }
        if let Some(v) = profile.torus_R { out.torus_R = v; }
        if let Some(v) = profile.torus_r { out.torus_r = v; }
        if let Some(v) = profile.time_step { out.time_step = v; }
        if let Some(v) = profile.seed { out.seed = Some(v); }
        if let Some(v) = profile.background_color { out.background_color = v; }
        if let Some(v) = profile.trail_mode { out.trail_mode = v; }
        if let Some(v) = profile.color_cycle_speed { out.color_cycle_speed = v; }
        if let Some(v) = profile.gradient_stops { out.gradient_stops = v; }
        if let Some(v) = profile.gradient_mode { out.gradient_mode = v; }
        if let Some(v) = profile.preset_cycle_secs { out.preset_cycle_secs = Some(v); }
        if let Some(v) = profile.presets_order { out.presets_order = v; }
        if let Some(v) = profile.catenoid_c { out.catenoid_c = v; }
        if let Some(v) = profile.helicoid_c { out.helicoid_c = v; }
        if let Some(v) = profile.hyperboloid_a { out.hyperboloid_a = v; }
        if let Some(v) = profile.hyperboloid_b { out.hyperboloid_b = v; }
        if let Some(v) = profile.light_dir { out.light_dir = v; }
        if let Some(v) = profile.color_cycle_enabled { out.color_cycle_enabled = v; }
        out
    }
}

/// Thread-safe handle to a [`Config`] that can be updated from a watcher thread.
pub type SharedConfig = Arc<RwLock<Config>>;

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        assert!(
            cfg.rotation_speed >= 0.0,
            "rotation_speed must be non-negative"
        );
        assert!(
            !cfg.color_palette.is_empty(),
            "color_palette must not be empty"
        );
        assert!(cfg.trail_length > 0, "trail_length must be > 0");
        // Surface name must be one of the known values.
        assert!(
            ["torus", "sphere", "saddle", "catenoid", "helicoid", "hyperboloid"]
                .contains(&cfg.surface.as_str()),
            "unexpected default surface: {}",
            cfg.surface
        );
    }

    /// Serialise the default config to TOML and deserialise it again; all
    /// fields must survive the round-trip unchanged.
    #[test]
    fn test_config_round_trip() {
        let original = Config::default();
        let toml_str = toml::to_string(&original).expect("serialization failed");
        let restored: Config = toml::from_str(&toml_str).expect("deserialization failed");

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
        assert!(
            cfg.num_geodesics > 0,
            "num_geodesics must be > 0, got {}",
            cfg.num_geodesics
        );
    }

    /// resolve_profile with no active_profile returns a clone of self.
    #[test]
    fn test_resolve_profile_no_active() {
        let cfg = Config::default();
        let resolved = cfg.resolve_profile();
        assert_eq!(cfg.surface, resolved.surface);
        assert_eq!(cfg.num_geodesics, resolved.num_geodesics);
    }

    /// resolve_profile with an active_profile that exists overlays its fields.
    #[test]
    fn test_resolve_profile_overlays_fields() {
        let mut cfg = Config::default();
        let mut profile = PartialConfig::default();
        profile.surface = Some("sphere".into());
        profile.num_geodesics = Some(5);
        cfg.profiles.insert("test".into(), profile);
        cfg.active_profile = Some("test".into());

        let resolved = cfg.resolve_profile();
        assert_eq!(resolved.surface, "sphere");
        assert_eq!(resolved.num_geodesics, 5);
        // Fields not in profile retain base config values.
        assert_eq!(resolved.trail_length, 300);
    }

    /// resolve_profile with a missing profile name returns a clone of self.
    #[test]
    fn test_resolve_profile_missing_profile() {
        let mut cfg = Config::default();
        cfg.active_profile = Some("nonexistent".into());
        let resolved = cfg.resolve_profile();
        assert_eq!(cfg.surface, resolved.surface);
    }

    /// New surface-specific config fields have correct defaults.
    #[test]
    fn test_new_surface_fields_defaults() {
        let cfg = Config::default();
        assert!((cfg.catenoid_c - 1.0).abs() < 1e-6);
        assert!((cfg.helicoid_c - 1.0).abs() < 1e-6);
        assert!((cfg.hyperboloid_a - 1.0).abs() < 1e-6);
        assert!((cfg.hyperboloid_b - 1.0).abs() < 1e-6);
        assert_eq!(cfg.light_dir, [1.0, 1.0, 1.0]);
        assert!(!cfg.color_cycle_enabled);
        assert_eq!(cfg.background_color, "#050510");
        assert_eq!(cfg.trail_mode, "line");
        assert!((cfg.color_cycle_speed - 0.0).abs() < 1e-6);
        assert_eq!(cfg.gradient_mode, "none");
        assert!(cfg.gradient_stops.is_empty());
        assert!(cfg.seed.is_none());
        assert!(cfg.active_profile.is_none());
        assert!(cfg.profiles.is_empty());
        assert!(cfg.preset_cycle_secs.is_none());
        assert!(cfg.presets_order.is_empty());
    }
}
