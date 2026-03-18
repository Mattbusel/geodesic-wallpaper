//! Integration tests for the `config` module.

use geodesic_wallpaper::config::Config;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn default_surface_is_torus() {
    let cfg = Config::default();
    assert_eq!(cfg.surface, "torus");
}

#[test]
fn default_num_geodesics_is_30() {
    let cfg = Config::default();
    assert_eq!(cfg.num_geodesics, 30);
}

#[test]
fn default_trail_length_is_300() {
    let cfg = Config::default();
    assert_eq!(cfg.trail_length, 300);
}

#[test]
fn default_time_step_is_positive() {
    let cfg = Config::default();
    assert!(cfg.time_step > 0.0);
}

#[test]
fn default_palette_has_five_entries() {
    let cfg = Config::default();
    assert_eq!(cfg.color_palette.len(), 5);
}

#[test]
fn config_load_from_tempfile() {
    let toml = b"surface = \"sphere\"\nnum_geodesics = 5\n";
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(toml).unwrap();
    let cfg = Config::load(f.path());
    assert_eq!(cfg.surface, "sphere");
    assert_eq!(cfg.num_geodesics, 5);
    // Unspecified fields fall back to defaults.
    assert_eq!(cfg.trail_length, 300);
}

#[test]
fn config_load_missing_file_returns_default() {
    let cfg = Config::load(std::path::Path::new("/no/such/file.toml"));
    assert_eq!(cfg.surface, "torus");
}

#[test]
fn config_load_invalid_toml_returns_default() {
    let toml = b"not valid toml :::";
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(toml).unwrap();
    let cfg = Config::load(f.path());
    assert_eq!(cfg.surface, "torus");
}

#[test]
fn parse_color_white() {
    let c = Config::parse_color("#FFFFFF");
    assert!((c[0] - 1.0).abs() < 0.01);
    assert!((c[1] - 1.0).abs() < 0.01);
    assert!((c[2] - 1.0).abs() < 0.01);
    assert_eq!(c[3], 1.0);
}

#[test]
fn parse_color_black() {
    let c = Config::parse_color("#000000");
    assert!(c[0].abs() < 0.01);
    assert!(c[1].abs() < 0.01);
    assert!(c[2].abs() < 0.01);
    assert_eq!(c[3], 1.0);
}

#[test]
fn parse_color_without_hash() {
    let with_hash = Config::parse_color("#4488FF");
    let without_hash = Config::parse_color("4488FF");
    for i in 0..4 {
        assert!((with_hash[i] - without_hash[i]).abs() < 1e-6);
    }
}

#[test]
fn parse_color_invalid_falls_back_to_midgray() {
    let c = Config::parse_color("#ZZZZZZ");
    let expected = 128.0 / 255.0;
    assert!((c[0] - expected).abs() < 0.01);
    assert!((c[1] - expected).abs() < 0.01);
    assert!((c[2] - expected).abs() < 0.01);
}

#[test]
fn parse_color_alpha_is_always_one() {
    for hex in ["#FF0000", "#00FF00", "#0000FF", "#FFFFFF", "#000000"] {
        assert_eq!(Config::parse_color(hex)[3], 1.0, "alpha != 1.0 for {hex}");
    }
}

#[test]
fn color_palette_default_entries_are_valid_hex() {
    let cfg = Config::default();
    for color in &cfg.color_palette {
        assert!(
            color.starts_with('#') && color.len() == 7,
            "unexpected palette entry: {color}"
        );
    }
}
