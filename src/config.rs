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
    /// Surface to render: `"torus"`, `"sphere"`, `"saddle"`, `"enneper"`, `"catenoid"`,
    /// `"helicoid"`, or `"hyperboloid"`.
    ///
    /// Any unrecognised value falls back to `"torus"`.
    #[serde(default = "default_surface")]
    pub surface: String,

    /// Number of simultaneous geodesic curves.
    ///
    /// Must be > 0. Default: `30`.
    #[serde(default = "default_num_geodesics")]
    pub num_geodesics: usize,

    /// Number of frames a trail persists before fading out.
    ///
    /// Must be > 0. Default: `300`.
    #[serde(default = "default_trail_length")]
    pub trail_length: usize,

    /// Camera orbit speed in radians per second.
    ///
    /// Default: `0.001047` (approximately one revolution every 100 minutes).
    #[serde(default = "default_rotation_speed")]
    pub rotation_speed: f32,

    /// Trail colour palette as CSS hex strings (e.g. `"#4488FF"`).
    ///
    /// Geodesics cycle through this list in `"cycle"` mode, or colours are
    /// assigned randomly in `"random"` mode. At least one colour is required;
    /// the default palette contains five entries.
    #[serde(default = "default_color_palette")]
    pub color_palette: Vec<String>,

    /// Torus major radius: distance from the torus center to the tube center.
    ///
    /// Must be > `torus_r`. Default: `2.0`.
    #[serde(default = "default_torus_r_big")]
    #[allow(non_snake_case)]
    pub torus_R: f32,

    /// Torus minor radius: tube radius.
    ///
    /// Must be > 0 and < `torus_R`. Default: `0.7`.
    #[serde(default = "default_torus_r_small")]
    pub torus_r: f32,

    /// RK4 integration timestep per frame in seconds.
    ///
    /// Default: `0.016`.
    #[serde(default = "default_time_step")]
    pub time_step: f32,

    // ── Camera ─────────────────────────────────────────────────────────────

    /// Distance from the origin to the camera eye.
    ///
    /// Default: `6.0`.
    #[serde(default = "default_camera_distance")]
    pub camera_distance: f32,

    /// Camera elevation above the XY plane in radians.
    ///
    /// Default: `0.4`.
    #[serde(default = "default_camera_elevation")]
    pub camera_elevation: f32,

    /// Vertical field-of-view in radians.
    ///
    /// Default: `0.8`.
    #[serde(default = "default_camera_fov")]
    pub camera_fov: f32,

    /// Speed at which the camera elevation drifts, in radians per second.
    ///
    /// Set to `0.0` to disable elevation drift. Elevation is clamped to
    /// `[0.05, 1.4]` and reverses at the limits. Default: `0.0`.
    #[serde(default = "default_camera_elevation_speed")]
    pub camera_elevation_speed: f32,

    // ── Rendering ──────────────────────────────────────────────────────────

    /// Whether to render the surface wireframe mesh.
    ///
    /// Default: `true`.
    #[serde(default = "default_show_wireframe")]
    pub show_wireframe: bool,

    /// Maximum number of trail vertices in the GPU buffer.
    ///
    /// Larger values consume more VRAM. Default: `100000`.
    #[serde(default = "default_max_trail_verts")]
    pub max_trail_verts: usize,

    /// Exponent for the trail alpha fade curve.
    ///
    /// `1.0` = linear, `2.0` = quadratic (default), `3.0` = cubic.
    /// Higher values make trails fade more sharply toward the tail.
    #[serde(default = "default_trail_fade_power")]
    pub trail_fade_power: f32,

    /// How to assign colours to geodesics.
    ///
    /// `"cycle"` (default) cycles through `color_palette` in order.
    /// `"random"` picks a random palette colour for each new geodesic.
    #[serde(default = "default_color_mode")]
    pub color_mode: String,

    /// Target frames per second for the render loop.
    ///
    /// Default: `30`.
    #[serde(default = "default_target_fps")]
    pub target_fps: u32,

    // ── UX ─────────────────────────────────────────────────────────────────

    /// Log FPS and surface info periodically to the console.
    ///
    /// Default: `false`.
    #[serde(default = "default_show_hud")]
    pub show_hud: bool,

    /// Show an epilepsy/photosensitivity warning on first launch.
    ///
    /// After displaying the warning once the application continues. Set to
    /// `false` to suppress. Default: `true`.
    #[serde(default = "default_epilepsy_warning")]
    pub epilepsy_warning: bool,

    /// Whether to span the wallpaper across all monitors (virtual screen).
    ///
    /// `false` (default) uses the primary monitor only.
    #[serde(default = "default_multi_monitor")]
    pub multi_monitor: bool,

    // ── New features branch additions ──────────────────────────────────────

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
fn default_camera_distance() -> f32 {
    6.0
}
fn default_camera_elevation() -> f32 {
    0.4
}
fn default_camera_fov() -> f32 {
    0.8
}
fn default_camera_elevation_speed() -> f32 {
    0.0
}
fn default_show_wireframe() -> bool {
    true
}
fn default_max_trail_verts() -> usize {
    100_000
}
fn default_trail_fade_power() -> f32 {
    2.0
}
fn default_color_mode() -> String {
    "cycle".into()
}
fn default_target_fps() -> u32 {
    30
}
fn default_show_hud() -> bool {
    false
}
fn default_epilepsy_warning() -> bool {
    true
}
fn default_multi_monitor() -> bool {
    false
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
    pub camera_distance: Option<f32>,
    pub camera_elevation: Option<f32>,
    pub camera_fov: Option<f32>,
    pub camera_elevation_speed: Option<f32>,
    pub show_wireframe: Option<bool>,
    pub max_trail_verts: Option<usize>,
    pub trail_fade_power: Option<f32>,
    pub color_mode: Option<String>,
    pub target_fps: Option<u32>,
    pub show_hud: Option<bool>,
    pub epilepsy_warning: Option<bool>,
    pub multi_monitor: Option<bool>,
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
            camera_distance: default_camera_distance(),
            camera_elevation: default_camera_elevation(),
            camera_fov: default_camera_fov(),
            camera_elevation_speed: default_camera_elevation_speed(),
            show_wireframe: default_show_wireframe(),
            max_trail_verts: default_max_trail_verts(),
            trail_fade_power: default_trail_fade_power(),
            color_mode: default_color_mode(),
            target_fps: default_target_fps(),
            show_hud: default_show_hud(),
            epilepsy_warning: default_epilepsy_warning(),
            multi_monitor: default_multi_monitor(),
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
    /// If the file cannot be read, the default config is returned silently
    /// (the file is optional). If the file exists but cannot be parsed, a
    /// warning is emitted via `tracing` and the error message is printed to
    /// `stderr` so users can diagnose `config.toml` syntax issues.
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
            Ok(s) => match toml::from_str::<Config>(&s) {
                Ok(cfg) => {
                    let warnings = cfg.validate();
                    for w in &warnings {
                        tracing::warn!("Config validation: {w}");
                        eprintln!("[geodesic-wallpaper] config warning: {w}");
                    }
                    cfg
                }
                Err(e) => {
                    let msg = format!("Config parse error in '{}': {e}", path.display());
                    tracing::warn!("{msg}");
                    eprintln!("[geodesic-wallpaper] {msg}");
                    eprintln!("[geodesic-wallpaper] using default configuration");
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    /// Validate configuration values and return a list of human-readable warnings.
    ///
    /// This does not modify the config; callers should log or display the
    /// returned messages. An empty `Vec` means the config is fully valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use geodesic_wallpaper::config::Config;
    ///
    /// let cfg = Config::default();
    /// assert!(cfg.validate().is_empty());
    /// ```
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.num_geodesics == 0 {
            warnings.push(
                "num_geodesics is 0 — no geodesics will be rendered; set to at least 1".into(),
            );
        }
        if self.trail_length == 0 {
            warnings.push(
                "trail_length is 0 — trails will be invisible; set to at least 1".into(),
            );
        }
        if self.torus_R <= 0.0 {
            warnings.push(format!(
                "torus_R = {} is not positive — torus will degenerate",
                self.torus_R
            ));
        }
        if self.torus_r <= 0.0 {
            warnings.push(format!(
                "torus_r = {} is not positive — torus will degenerate",
                self.torus_r
            ));
        }
        if self.torus_R > 0.0 && self.torus_r >= self.torus_R {
            warnings.push(format!(
                "torus_r ({}) >= torus_R ({}) — torus will self-intersect",
                self.torus_r, self.torus_R
            ));
        }
        if self.time_step <= 0.0 {
            warnings.push(format!(
                "time_step = {} is not positive — simulation will not advance",
                self.time_step
            ));
        }
        if self.color_palette.is_empty() {
            warnings.push(
                "color_palette is empty — a fallback grey colour will be used".into(),
            );
        }
        if self.max_trail_verts < 100 {
            warnings.push(format!(
                "max_trail_verts = {} is very small — trails may be clipped",
                self.max_trail_verts
            ));
        }
        if self.target_fps == 0 {
            warnings.push("target_fps is 0 — defaulting to 30 fps".into());
        }
        if self.trail_fade_power <= 0.0 {
            warnings.push(format!(
                "trail_fade_power = {} must be positive; defaulting to 2.0",
                self.trail_fade_power
            ));
        }
        if !["cycle", "random"].contains(&self.color_mode.as_str()) {
            warnings.push(format!(
                "color_mode '{}' is unrecognised — use 'cycle' or 'random'; defaulting to 'cycle'",
                self.color_mode
            ));
        }
        let known_surfaces = [
            "torus", "sphere", "saddle", "enneper", "catenoid", "helicoid", "hyperboloid",
        ];
        if !known_surfaces.contains(&self.surface.as_str()) {
            warnings.push(format!(
                "surface '{}' is unrecognised — known values: {}; defaulting to 'torus'",
                self.surface,
                known_surfaces.join(", ")
            ));
        }

        warnings
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

    /// Return the effective target FPS, clamping zero to 30.
    pub fn effective_target_fps(&self) -> u32 {
        if self.target_fps == 0 { 30 } else { self.target_fps }
    }

    /// Return the effective trail fade power, clamping non-positive values to 2.0.
    pub fn effective_fade_power(&self) -> f32 {
        if self.trail_fade_power <= 0.0 { 2.0 } else { self.trail_fade_power }
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
        if let Some(v) = profile.camera_distance { out.camera_distance = v; }
        if let Some(v) = profile.camera_elevation { out.camera_elevation = v; }
        if let Some(v) = profile.camera_fov { out.camera_fov = v; }
        if let Some(v) = profile.camera_elevation_speed { out.camera_elevation_speed = v; }
        if let Some(v) = profile.show_wireframe { out.show_wireframe = v; }
        if let Some(v) = profile.max_trail_verts { out.max_trail_verts = v; }
        if let Some(v) = profile.trail_fade_power { out.trail_fade_power = v; }
        if let Some(v) = profile.color_mode { out.color_mode = v; }
        if let Some(v) = profile.target_fps { out.target_fps = v; }
        if let Some(v) = profile.show_hud { out.show_hud = v; }
        if let Some(v) = profile.epilepsy_warning { out.epilepsy_warning = v; }
        if let Some(v) = profile.multi_monitor { out.multi_monitor = v; }
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
        assert!((cfg.camera_distance - 6.0).abs() < 1e-6);
        assert!((cfg.camera_elevation - 0.4).abs() < 1e-6);
        assert!((cfg.camera_fov - 0.8).abs() < 1e-6);
        assert_eq!(cfg.show_wireframe, true);
        assert_eq!(cfg.max_trail_verts, 100_000);
        assert!((cfg.trail_fade_power - 2.0).abs() < 1e-6);
        assert_eq!(cfg.color_mode, "cycle");
        assert_eq!(cfg.target_fps, 30);
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
camera_distance = 8.0
camera_elevation = 0.6
camera_fov = 1.0
show_wireframe = false
max_trail_verts = 50000
trail_fade_power = 3.0
color_mode = "random"
target_fps = 60
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
        assert!((cfg.camera_distance - 8.0).abs() < 1e-6);
        assert!((cfg.camera_elevation - 0.6).abs() < 1e-6);
        assert!((cfg.camera_fov - 1.0).abs() < 1e-6);
        assert_eq!(cfg.show_wireframe, false);
        assert_eq!(cfg.max_trail_verts, 50000);
        assert!((cfg.trail_fade_power - 3.0).abs() < 1e-6);
        assert_eq!(cfg.color_mode, "random");
        assert_eq!(cfg.target_fps, 60);
        assert_eq!(cfg.color_palette, vec!["#FF0000"]);
    }

    #[test]
    fn partial_config_falls_back_to_defaults() {
        let toml = r#"surface = "saddle""#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml.as_bytes()).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "saddle");
        assert_eq!(cfg.num_geodesics, 30);
        assert_eq!(cfg.trail_length, 300);
        assert_eq!(cfg.show_wireframe, true);
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
        let c = Config::parse_color("#ZZZZZZ");
        assert!((c[0] - 128.0 / 255.0).abs() < 0.01);
        assert_eq!(c[3], 1.0);
    }

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
        let known = [
            "torus", "sphere", "saddle", "enneper", "catenoid", "helicoid", "hyperboloid",
        ];
        assert!(
            known.contains(&cfg.surface.as_str()),
            "unexpected default surface: {}",
            cfg.surface
        );
        assert!(cfg.validate().is_empty(), "default config should have no warnings");
    }

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
        assert_eq!(original.show_wireframe, restored.show_wireframe);
        assert_eq!(original.max_trail_verts, restored.max_trail_verts);
        assert!((original.trail_fade_power - restored.trail_fade_power).abs() < 1e-9);
        assert_eq!(original.color_mode, restored.color_mode);
        assert_eq!(original.target_fps, restored.target_fps);
    }

    #[test]
    fn test_config_geodesic_count_nonzero() {
        let cfg = Config::default();
        assert!(
            cfg.num_geodesics > 0,
            "num_geodesics must be > 0, got {}",
            cfg.num_geodesics
        );
    }

    #[test]
    fn validate_catches_zero_geodesics() {
        let mut cfg = Config::default();
        cfg.num_geodesics = 0;
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("num_geodesics")),
            "expected warning about num_geodesics, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_catches_self_intersecting_torus() {
        let mut cfg = Config::default();
        cfg.torus_r = 3.0;
        cfg.torus_R = 2.0;
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("self-intersect")),
            "expected self-intersect warning, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_catches_empty_palette() {
        let mut cfg = Config::default();
        cfg.color_palette = vec![];
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("color_palette")),
            "expected palette warning, got: {warnings:?}"
        );
    }

    #[test]
    fn effective_target_fps_clamps_zero() {
        let mut cfg = Config::default();
        cfg.target_fps = 0;
        assert_eq!(cfg.effective_target_fps(), 30);
        cfg.target_fps = 60;
        assert_eq!(cfg.effective_target_fps(), 60);
    }

    #[test]
    fn effective_fade_power_clamps_nonpositive() {
        let mut cfg = Config::default();
        cfg.trail_fade_power = 0.0;
        assert!((cfg.effective_fade_power() - 2.0).abs() < 1e-6);
        cfg.trail_fade_power = 3.0;
        assert!((cfg.effective_fade_power() - 3.0).abs() < 1e-6);
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
