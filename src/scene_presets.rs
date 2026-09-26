//! Scene Presets
//!
//! Named configurations combining surface type, colour scheme, geodesic
//! parameters, and transition settings.  The [`PresetLibrary`] ships with
//! 10 built-in presets and supports sequential navigation via [`PresetLibrary::next`]
//! and [`PresetLibrary::prev`].
//!
//! # Example
//!
//! ```ignore
//! use geodesic_wallpaper::scene_presets::PresetLibrary;
//!
//! let mut lib = PresetLibrary::with_defaults();
//! assert!(lib.count() >= 8);
//! let preset = lib.current();
//! println!("Current preset: {}", preset.name);
//!
//! let next = lib.next();
//! assert_ne!(next.name, lib.prev().name);
//! ```

/// A complete scene configuration.
#[derive(Debug, Clone)]
pub struct ScenePreset {
    /// Short identifier, e.g. `"cosmic"`.
    pub name: String,
    /// Surface type string accepted by the `Config::surface` field.
    pub surface: String,
    /// Background clear colour as linear RGBA in `[0.0, 1.0]`.
    pub background_color: [f32; 4],
    /// Colour at the *head* of each geodesic trail (most opaque end).
    pub trail_color_start: [f32; 4],
    /// Colour at the *tail* of each geodesic trail (fades out here).
    pub trail_color_end: [f32; 4],
    /// Number of simultaneous geodesic curves.
    pub geodesic_count: usize,
    /// Speed multiplier applied to the RK4 integrator.
    pub speed: f32,
    /// Trail length in frames before a curve is respawned.
    pub trail_length: usize,
    /// Human-readable description shown in the UI overlay.
    pub description: String,
}

impl ScenePreset {
    /// Convert the preset's `trail_color_start` to a CSS hex string like `"#4488FF"`.
    pub fn trail_start_hex(&self) -> String {
        let [r, g, b, _] = self.trail_color_start;
        format!(
            "#{:02X}{:02X}{:02X}",
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8
        )
    }

    /// Convert the preset's `background_color` to a CSS hex string.
    pub fn background_hex(&self) -> String {
        let [r, g, b, _] = self.background_color;
        format!(
            "#{:02X}{:02X}{:02X}",
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8
        )
    }
}

/// An ordered library of [`ScenePreset`]s with a cursor for sequential navigation.
pub struct PresetLibrary {
    presets: Vec<ScenePreset>,
    current_idx: usize,
}

impl PresetLibrary {
    /// Construct the library pre-loaded with all built-in presets.
    pub fn with_defaults() -> Self {
        Self {
            presets: default_presets(),
            current_idx: 0,
        }
    }

    /// Build a library from a custom list of presets.
    ///
    /// # Panics
    ///
    /// Panics if `presets` is empty.
    pub fn from_presets(presets: Vec<ScenePreset>) -> Self {
        assert!(
            !presets.is_empty(),
            "PresetLibrary requires at least one preset"
        );
        Self {
            presets,
            current_idx: 0,
        }
    }

    /// Advance the cursor and return a reference to the next preset.
    ///
    /// Wraps around to the first preset after the last.
    pub fn next(&mut self) -> &ScenePreset {
        self.current_idx = (self.current_idx + 1) % self.presets.len();
        &self.presets[self.current_idx]
    }

    /// Move the cursor backwards and return a reference to the previous preset.
    ///
    /// Wraps around to the last preset before the first.
    pub fn prev(&mut self) -> &ScenePreset {
        if self.current_idx == 0 {
            self.current_idx = self.presets.len() - 1;
        } else {
            self.current_idx -= 1;
        }
        &self.presets[self.current_idx]
    }

    /// Look up a preset by name (case-insensitive).
    pub fn by_name(&self, name: &str) -> Option<&ScenePreset> {
        let lower = name.to_lowercase();
        self.presets.iter().find(|p| p.name.to_lowercase() == lower)
    }

    /// Return a reference to the currently selected preset.
    pub fn current(&self) -> &ScenePreset {
        &self.presets[self.current_idx]
    }

    /// Return the total number of presets in this library.
    pub fn count(&self) -> usize {
        self.presets.len()
    }

    /// Return an iterator over all presets in library order.
    pub fn iter(&self) -> impl Iterator<Item = &ScenePreset> {
        self.presets.iter()
    }
}

// ── Built-in preset definitions ──────────────────────────────────────────────

fn default_presets() -> Vec<ScenePreset> {
    vec![
        // 1. Cosmic — deep-space purples and gold traces on a torus.
        ScenePreset {
            name: "cosmic".into(),
            surface: "torus".into(),
            background_color: [0.02, 0.01, 0.08, 1.0],
            trail_color_start: [1.0, 0.84, 0.0, 1.0], // gold
            trail_color_end: [0.27, 0.0, 0.51, 0.0],  // deep purple, faded
            geodesic_count: 35,
            speed: 1.0,
            trail_length: 350,
            description: "Deep-space aesthetic: gold geodesics tracing the torus through a \
                          near-black indigo void."
                .into(),
        },
        // 2. Ocean — blue-green traces on a sphere.
        ScenePreset {
            name: "ocean".into(),
            surface: "sphere".into(),
            background_color: [0.0, 0.04, 0.12, 1.0],
            trail_color_start: [0.0, 0.85, 0.9, 1.0], // cyan
            trail_color_end: [0.0, 0.2, 0.5, 0.0],    // deep blue, faded
            geodesic_count: 28,
            speed: 0.8,
            trail_length: 400,
            description: "Ocean depths: great-circle geodesics flow across a sphere in \
                          cool aqua tones."
                .into(),
        },
        // 3. Ember — warm reds and oranges on a saddle.
        ScenePreset {
            name: "ember".into(),
            surface: "saddle".into(),
            background_color: [0.05, 0.01, 0.0, 1.0],
            trail_color_start: [1.0, 0.45, 0.0, 1.0], // orange
            trail_color_end: [0.6, 0.05, 0.0, 0.0],   // dark red, faded
            geodesic_count: 25,
            speed: 1.2,
            trail_length: 280,
            description: "Ember glow: fiery geodesics streak across the hyperbolic saddle \
                          surface."
                .into(),
        },
        // 4. Forest — deep greens on a hyperboloid.
        ScenePreset {
            name: "forest".into(),
            surface: "hyperboloid".into(),
            background_color: [0.01, 0.05, 0.01, 1.0],
            trail_color_start: [0.2, 0.9, 0.3, 1.0], // bright green
            trail_color_end: [0.02, 0.25, 0.04, 0.0], // dark green, faded
            geodesic_count: 30,
            speed: 0.9,
            trail_length: 320,
            description: "Forest canopy: lush green geodesics trace the one-sheeted \
                          hyperboloid."
                .into(),
        },
        // 5. Aurora — arctic blues and greens on a torus knot.
        ScenePreset {
            name: "aurora".into(),
            surface: "torus_knot".into(),
            background_color: [0.0, 0.02, 0.06, 1.0],
            trail_color_start: [0.0, 1.0, 0.7, 1.0], // aurora green
            trail_color_end: [0.0, 0.4, 0.9, 0.0],   // aurora blue, faded
            geodesic_count: 22,
            speed: 0.7,
            trail_length: 450,
            description: "Aurora borealis: shimmering green-blue traces along the trefoil \
                          torus-knot tube."
                .into(),
        },
        // 6. Neon — hot pink and electric blue on a Klein bottle.
        ScenePreset {
            name: "neon".into(),
            surface: "klein_bottle".into(),
            background_color: [0.0, 0.0, 0.0, 1.0],
            trail_color_start: [1.0, 0.07, 0.57, 1.0], // hot pink
            trail_color_end: [0.0, 0.5, 1.0, 0.0],     // electric blue, faded
            geodesic_count: 40,
            speed: 1.4,
            trail_length: 250,
            description: "Neon city: high-contrast pink and blue geodesics on the \
                          non-orientable Klein bottle."
                .into(),
        },
        // 7. Dusk — warm pinks and purples on an ellipsoid.
        ScenePreset {
            name: "dusk".into(),
            surface: "ellipsoid".into(),
            background_color: [0.06, 0.02, 0.04, 1.0],
            trail_color_start: [0.95, 0.5, 0.7, 1.0], // rose pink
            trail_color_end: [0.35, 0.05, 0.35, 0.0], // deep purple, faded
            geodesic_count: 30,
            speed: 0.85,
            trail_length: 380,
            description: "Twilight: rose-to-purple geodesics sweep across the ellipsoidal \
                          surface at dusk."
                .into(),
        },
        // 8. Ice — cold whites and silvers on a catenoid.
        ScenePreset {
            name: "ice".into(),
            surface: "catenoid".into(),
            background_color: [0.02, 0.04, 0.08, 1.0],
            trail_color_start: [0.85, 0.95, 1.0, 1.0], // ice white-blue
            trail_color_end: [0.3, 0.5, 0.7, 0.0],     // steel blue, faded
            geodesic_count: 26,
            speed: 1.0,
            trail_length: 350,
            description: "Ice cave: crystal-blue geodesics spiral around the catenoid \
                          waist."
                .into(),
        },
        // 9. Lava — high-contrast reds and yellows on a Boy surface.
        ScenePreset {
            name: "lava".into(),
            surface: "boy_surface".into(),
            background_color: [0.04, 0.0, 0.0, 1.0],
            trail_color_start: [1.0, 0.9, 0.1, 1.0], // molten yellow
            trail_color_end: [0.7, 0.05, 0.0, 0.0],  // deep red, faded
            geodesic_count: 32,
            speed: 1.3,
            trail_length: 300,
            description: "Volcanic flow: molten yellow-to-red geodesics on the \
                          three-fold-symmetric Boy surface."
                .into(),
        },
        // 10. Void — nearly invisible dark cyan on an Enneper surface.
        ScenePreset {
            name: "void".into(),
            surface: "enneper".into(),
            background_color: [0.0, 0.0, 0.02, 1.0],
            trail_color_start: [0.1, 0.6, 0.55, 1.0], // teal
            trail_color_end: [0.0, 0.1, 0.15, 0.0],   // near-black teal, faded
            geodesic_count: 20,
            speed: 0.6,
            trail_length: 500,
            description: "Void: slow teal geodesics fade almost to nothing on the \
                          Enneper minimal surface."
                .into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_ten_presets() {
        let lib = PresetLibrary::with_defaults();
        assert_eq!(lib.count(), 10);
    }

    #[test]
    fn next_wraps_around() {
        let mut lib = PresetLibrary::with_defaults();
        let first = lib.current().name.clone();
        for _ in 0..lib.count() {
            lib.next();
        }
        assert_eq!(lib.current().name, first);
    }

    #[test]
    fn prev_wraps_around() {
        let mut lib = PresetLibrary::with_defaults();
        let first = lib.current().name.clone();
        lib.prev();
        assert_ne!(lib.current().name, first);
        // Going forward again should return to first.
        lib.next();
        assert_eq!(lib.current().name, first);
    }

    #[test]
    fn by_name_case_insensitive() {
        let lib = PresetLibrary::with_defaults();
        assert!(lib.by_name("COSMIC").is_some());
        assert!(lib.by_name("ocean").is_some());
        assert!(lib.by_name("doesnotexist").is_none());
    }

    #[test]
    fn trail_start_hex_format() {
        let lib = PresetLibrary::with_defaults();
        let hex = lib.current().trail_start_hex();
        assert!(hex.starts_with('#'));
        assert_eq!(hex.len(), 7);
    }

    #[test]
    fn all_presets_have_names() {
        let lib = PresetLibrary::with_defaults();
        for p in lib.iter() {
            assert!(!p.name.is_empty());
            assert!(!p.surface.is_empty());
            assert!(!p.description.is_empty());
        }
    }

    #[test]
    fn from_presets_rejects_empty() {
        let result = std::panic::catch_unwind(|| PresetLibrary::from_presets(vec![]));
        assert!(result.is_err());
    }
}
