//! Live parameter tuning via keyboard shortcuts.
//!
//! The `[tuning]` section of `config.toml` lists parameters that can be
//! adjusted while the wallpaper is running.  For example:
//!
//! ```toml
//! [tuning]
//! [[tuning.parameters]]
//! name    = "rotation_speed"
//! min     = 0.0
//! max     = 0.1
//! current = 0.001047
//! step    = 0.0001
//!
//! [[tuning.parameters]]
//! name    = "trail_fade_power"
//! min     = 0.5
//! max     = 5.0
//! current = 2.0
//! step    = 0.1
//! ```
//!
//! Keyboard shortcuts (active when the wallpaper window has focus):
//!
//! | Key | Action |
//! |-----|--------|
//! | `[` | Select previous parameter |
//! | `]` | Select next parameter |
//! | `-` | Decrease current value by one step |
//! | `=` | Increase current value by one step |
//!
//! When the application exits the updated values are written back to
//! `config.toml` so they persist across restarts.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

// ─── Data types ───────────────────────────────────────────────────────────────

/// A single tunable parameter with its range and current value.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ParameterRange {
    /// The config field name this parameter maps to (e.g. `"rotation_speed"`).
    pub name: String,
    /// Minimum allowed value.
    pub min: f64,
    /// Maximum allowed value.
    pub max: f64,
    /// Current value.  Clamped to `[min, max]` on load.
    pub current: f64,
    /// Amount to add/subtract on each key press.
    pub step: f64,
}

impl ParameterRange {
    /// Construct a new parameter range, clamping `current` into `[min, max]`.
    pub fn new(name: impl Into<String>, min: f64, max: f64, current: f64, step: f64) -> Self {
        let current = current.clamp(min, max);
        Self { name: name.into(), min, max, current, step }
    }

    /// Increment the value by one step, clamping to `max`.
    pub fn increment(&mut self) {
        self.current = (self.current + self.step).min(self.max);
    }

    /// Decrement the value by one step, clamping to `min`.
    pub fn decrement(&mut self) {
        self.current = (self.current - self.step).max(self.min);
    }

    /// Returns a human-readable overlay string, e.g. `"rotation_speed: 0.0020"`.
    pub fn overlay_text(&self) -> String {
        format!("{}: {:.4}", self.name, self.current)
    }
}

// ─── Tuning section TOML schema ───────────────────────────────────────────────

/// The `[tuning]` section as it appears in `config.toml`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TuningConfig {
    /// List of tunable parameters.
    #[serde(default)]
    pub parameters: Vec<ParameterRange>,
}

// ─── ParameterTuner ───────────────────────────────────────────────────────────

/// Live parameter tuner that manages a list of [`ParameterRange`] values and
/// handles keyboard-driven increments / decrements.
pub struct ParameterTuner {
    params: Vec<ParameterRange>,
    selected: usize,
    config_path: PathBuf,
}

impl ParameterTuner {
    /// Load tunable parameters from the `[tuning]` section of `config_path`.
    ///
    /// If the file cannot be read or the `[tuning]` section is absent the tuner
    /// starts with an empty parameter list.
    pub fn from_config(config_path: &Path) -> Self {
        let params = Self::load_params(config_path).unwrap_or_default();
        Self {
            params,
            selected: 0,
            config_path: config_path.to_owned(),
        }
    }

    /// Return the index of the currently-selected parameter.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Return a reference to all parameters.
    pub fn parameters(&self) -> &[ParameterRange] {
        &self.params
    }

    /// Return the currently-selected parameter, if any.
    pub fn current_param(&self) -> Option<&ParameterRange> {
        self.params.get(self.selected)
    }

    /// Overlay text for the UI: parameter name and value.
    pub fn overlay_text(&self) -> Option<String> {
        self.current_param().map(|p| p.overlay_text())
    }

    // ── Keyboard handlers ─────────────────────────────────────────────────────

    /// Select the previous parameter (`[` key).
    pub fn select_prev(&mut self) {
        if self.params.is_empty() {
            return;
        }
        if self.selected == 0 {
            self.selected = self.params.len() - 1;
        } else {
            self.selected -= 1;
        }
    }

    /// Select the next parameter (`]` key).
    pub fn select_next(&mut self) {
        if self.params.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.params.len();
    }

    /// Decrease the selected parameter by one step (`-` key).
    pub fn decrease(&mut self) {
        if let Some(p) = self.params.get_mut(self.selected) {
            p.decrement();
        }
    }

    /// Increase the selected parameter by one step (`=` key).
    pub fn increase(&mut self) {
        if let Some(p) = self.params.get_mut(self.selected) {
            p.increment();
        }
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Persist changed values back to `config.toml`.
    ///
    /// Only the `[tuning]` section is updated; all other keys are preserved.
    /// Returns `Ok(())` on success; errors are non-fatal (logged via `tracing`).
    pub fn persist(&self) -> Result<(), PersistError> {
        let raw = std::fs::read_to_string(&self.config_path)
            .unwrap_or_default();

        // Strip the existing [tuning] block if present.
        let stripped = strip_tuning_section(&raw);

        // Serialise the tuning section.
        let tuning = TuningConfig { parameters: self.params.clone() };
        let tuning_toml = toml::to_string_pretty(&tuning)
            .map_err(PersistError::Serialize)?;

        let new_content = if tuning_toml.trim().is_empty() {
            stripped
        } else {
            format!("{}\n[tuning]\n{}", stripped.trim_end(), tuning_toml)
        };

        std::fs::write(&self.config_path, new_content)
            .map_err(PersistError::Io)?;

        Ok(())
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn load_params(path: &Path) -> Option<Vec<ParameterRange>> {
        let raw = std::fs::read_to_string(path).ok()?;
        // Parse just the [tuning] section.
        let full: toml::Value = toml::from_str(&raw).ok()?;
        let tuning_table = full.get("tuning")?;
        let tuning: TuningConfig = tuning_table.clone().try_into().ok()?;
        // Clamp all values into their declared ranges.
        let params = tuning.parameters.into_iter().map(|mut p| {
            p.current = p.current.clamp(p.min, p.max);
            p
        }).collect();
        Some(params)
    }
}

/// Errors that can occur when persisting parameter values to `config.toml`.
#[derive(Debug)]
pub enum PersistError {
    /// IO error writing the file.
    Io(io::Error),
    /// TOML serialisation error.
    Serialize(toml::ser::Error),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "IO error: {e}"),
            PersistError::Serialize(e) => write!(f, "TOML serialize error: {e}"),
        }
    }
}

impl std::error::Error for PersistError {}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Remove the `[tuning]` section and all lines belonging to it from a TOML
/// document (returned as a `String`).
fn strip_tuning_section(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_tuning = false;
    for line in src.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            // New section header — check if it's [tuning] or a subsection.
            in_tuning = trimmed == "[tuning]"
                || trimmed.starts_with("[tuning.")
                || trimmed.starts_with("[[tuning");
        }
        if !in_tuning {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_param(current: f64) -> ParameterRange {
        ParameterRange::new("speed", 0.0, 1.0, current, 0.1)
    }

    #[test]
    fn increment_clamps_to_max() {
        let mut p = make_param(0.95);
        p.increment();
        assert!((p.current - 1.0).abs() < 1e-10, "should clamp to max");
        p.increment(); // already at max
        assert!((p.current - 1.0).abs() < 1e-10);
    }

    #[test]
    fn decrement_clamps_to_min() {
        let mut p = make_param(0.05);
        p.decrement();
        assert!(p.current.abs() < 1e-10, "should clamp to min");
        p.decrement();
        assert!(p.current.abs() < 1e-10);
    }

    #[test]
    fn new_clamps_current() {
        let p = ParameterRange::new("x", 0.5, 1.0, 0.0, 0.1);
        assert!((p.current - 0.5).abs() < 1e-10);
    }

    #[test]
    fn overlay_text_format() {
        let p = ParameterRange::new("rotation_speed", 0.0, 1.0, 0.002, 0.001);
        assert_eq!(p.overlay_text(), "rotation_speed: 0.0020");
    }

    #[test]
    fn tuner_select_next_wraps() {
        let mut tuner = ParameterTuner {
            params: vec![
                ParameterRange::new("a", 0.0, 1.0, 0.5, 0.1),
                ParameterRange::new("b", 0.0, 1.0, 0.5, 0.1),
            ],
            selected: 0,
            config_path: PathBuf::from("config.toml"),
        };
        tuner.select_next();
        assert_eq!(tuner.selected, 1);
        tuner.select_next();
        assert_eq!(tuner.selected, 0, "should wrap back to 0");
    }

    #[test]
    fn tuner_select_prev_wraps() {
        let mut tuner = ParameterTuner {
            params: vec![
                ParameterRange::new("a", 0.0, 1.0, 0.5, 0.1),
                ParameterRange::new("b", 0.0, 1.0, 0.5, 0.1),
            ],
            selected: 0,
            config_path: PathBuf::from("config.toml"),
        };
        tuner.select_prev();
        assert_eq!(tuner.selected, 1, "should wrap to last");
    }

    #[test]
    fn tuner_empty_params_no_panic() {
        let mut tuner = ParameterTuner {
            params: vec![],
            selected: 0,
            config_path: PathBuf::from("config.toml"),
        };
        tuner.select_next();
        tuner.select_prev();
        tuner.increase();
        tuner.decrease();
        assert!(tuner.overlay_text().is_none());
    }

    #[test]
    fn strip_tuning_section_removes_block() {
        let src = "[foo]\na=1\n[tuning]\nb=2\n[[tuning.parameters]]\nname=\"x\"\n[bar]\nc=3\n";
        let stripped = strip_tuning_section(src);
        assert!(stripped.contains("[foo]"), "should keep [foo]");
        assert!(stripped.contains("[bar]"), "should keep [bar]");
        assert!(!stripped.contains("[tuning]"), "should remove [tuning]");
        assert!(!stripped.contains("[[tuning.parameters]]"), "should remove tuning sub-table");
    }

    #[test]
    fn persist_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "surface = \"torus\"\n").unwrap();

        let mut tuner = ParameterTuner::from_config(&path);
        // Manually add a parameter since the config has no [tuning] section.
        tuner.params.push(ParameterRange::new("rotation_speed", 0.0, 1.0, 0.5, 0.1));
        tuner.increase(); // 0.5 + 0.1 = 0.6
        tuner.persist().unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("[tuning]"), "persisted config should have [tuning]");
        assert!(content.contains("rotation_speed"), "should contain parameter name");
        assert!(content.contains("surface"), "should preserve original keys");
    }
}
