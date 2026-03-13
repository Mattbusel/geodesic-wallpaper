use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_surface")]
    pub surface: String,
    #[serde(default = "default_num_geodesics")]
    pub num_geodesics: usize,
    #[serde(default = "default_trail_length")]
    pub trail_length: usize,
    #[serde(default = "default_rotation_speed")]
    pub rotation_speed: f32,
    #[serde(default = "default_color_palette")]
    pub color_palette: Vec<String>,
    #[serde(default = "default_torus_r_big")]
    pub torus_R: f32,
    #[serde(default = "default_torus_r_small")]
    pub torus_r: f32,
}

fn default_surface() -> String { "torus".into() }
fn default_num_geodesics() -> usize { 30 }
fn default_trail_length() -> usize { 300 }
fn default_rotation_speed() -> f32 { 0.001047 }
fn default_color_palette() -> Vec<String> {
    vec!["#4488FF".into(), "#88DDFF".into(), "#FFD700".into(), "#88FF88".into(), "#FF88CC".into()]
}
fn default_torus_r_big() -> f32 { 2.0 }
fn default_torus_r_small() -> f32 { 0.7 }

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
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
                log::warn!("Config parse error: {e}, using defaults");
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn parse_color(hex: &str) -> [f32; 4] {
        let h = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(128) as f32 / 255.0;
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(128) as f32 / 255.0;
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(128) as f32 / 255.0;
        [r, g, b, 1.0]
    }
}

pub type SharedConfig = Arc<RwLock<Config>>;
