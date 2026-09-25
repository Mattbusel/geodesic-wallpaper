//! Multi-Monitor Support
//!
//! Assigns a different geodesic surface to each physical monitor.
//! Each monitor gets its own rendering context and configuration.
//!
//! # Example
//!
//! ```
//! use geodesic_wallpaper::multi_monitor::MultiMonitorManager;
//!
//! let manager = MultiMonitorManager::new();
//! assert!(manager.monitor_count() >= 1);
//!
//! let cfg = manager.config_for_monitor(0);
//! assert!(cfg.is_some());
//! ```

/// Per-monitor rendering configuration.
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Zero-based index of the physical monitor.
    pub monitor_index: usize,
    /// Surface type assigned to this monitor.
    /// Accepted values: `"torus"`, `"sphere"`, `"saddle"`, `"klein"`, `"hyperboloid"`,
    /// `"catenoid"`, `"helicoid"`, `"ellipsoid"`, `"enneper"`, `"boy_surface"`,
    /// `"hyperbolic_paraboloid"`, `"torus_knot"`.
    pub surface: String,
    /// Named colour scheme for this monitor (e.g. `"cosmic"`, `"ocean"`, `"ember"`).
    pub color_scheme: String,
    /// Number of simultaneous geodesic curves rendered on this monitor.
    pub geodesic_count: usize,
    /// Speed multiplier applied to the RK4 integrator for this monitor.
    pub speed: f32,
}

impl MonitorConfig {
    /// Create a `MonitorConfig` with sensible defaults for the given index.
    pub fn default_for_index(monitor_index: usize) -> Self {
        let surfaces = [
            "torus",
            "sphere",
            "saddle",
            "hyperboloid",
            "klein",
            "catenoid",
            "helicoid",
            "ellipsoid",
            "enneper",
            "boy_surface",
            "hyperbolic_paraboloid",
            "torus_knot",
        ];
        let schemes = [
            "cosmic", "ocean", "ember", "forest", "aurora", "neon", "dusk", "ice",
        ];

        MonitorConfig {
            monitor_index,
            surface: surfaces[monitor_index % surfaces.len()].to_string(),
            color_scheme: schemes[monitor_index % schemes.len()].to_string(),
            geodesic_count: 30,
            speed: 1.0,
        }
    }
}

/// Manages per-monitor surface assignments and configurations.
pub struct MultiMonitorManager {
    monitors: Vec<MonitorConfig>,
}

impl Default for MultiMonitorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiMonitorManager {
    /// Create a `MultiMonitorManager` by auto-detecting connected monitors
    /// and assigning a default surface to each one.
    pub fn new() -> Self {
        let names = Self::detect_monitors();
        let configs = Self::assign_surfaces(names.len());
        Self { monitors: configs }
    }

    /// Enumerate connected monitors and return their display names.
    ///
    /// On Windows this calls `EnumDisplayMonitors` via the `windows` crate if
    /// available; otherwise it falls back to a single-monitor stub so the rest
    /// of the codebase compiles on all platforms.
    pub fn detect_monitors() -> Vec<String> {
        // On Windows the real implementation would call:
        //   EnumDisplayMonitors(HDC::default(), None, Some(monitor_enum_proc), LPARAM(0))
        // For portability and testability we return a stub of one monitor.
        // Replace this body with the Win32 enumeration when integrating into
        // the wallpaper render loop.
        #[cfg(target_os = "windows")]
        {
            // Attempt to enumerate via windows crate; fall back to stub on error.
            Self::detect_monitors_win32().unwrap_or_else(|_| vec!["Monitor 0".to_string()])
        }
        #[cfg(not(target_os = "windows"))]
        {
            vec!["Monitor 0".to_string()]
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_monitors_win32() -> Result<Vec<String>, ()> {
        // Enumerate connected display monitors using Win32 GDI.
        // `EnumDisplayMonitors` calls the callback once per monitor.
        use std::sync::{Arc, Mutex};
        use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
        use windows::Win32::Graphics::Gdi::{
            EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
        };

        let names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let names_ptr = Arc::clone(&names);

        unsafe extern "system" fn monitor_enum_proc(
            _hmonitor: HMONITOR,
            _hdc: HDC,
            _rect: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let counter = lparam.0 as *mut usize;
            let idx = unsafe { *counter };
            unsafe { *counter += 1 };
            let _ = idx; // used below via closure
            BOOL(1)
        }

        let mut counter: usize = 0;
        let counter_ptr = &mut counter as *mut usize;

        unsafe {
            let _ = EnumDisplayMonitors(
                HDC::default(),
                None,
                Some(monitor_enum_proc),
                LPARAM(counter_ptr as isize),
            );
        }

        let count = counter.max(1);
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            result.push(format!("Monitor {i}"));
        }
        *names_ptr.lock().unwrap() = result.clone();
        Ok(result)
    }

    /// Build a `Vec<MonitorConfig>` assigning a distinct surface to each monitor.
    ///
    /// Surfaces cycle through the full built-in surface list so that even a
    /// 12-monitor setup has a unique surface per display.
    pub fn assign_surfaces(monitor_count: usize) -> Vec<MonitorConfig> {
        let count = monitor_count.max(1);
        (0..count).map(MonitorConfig::default_for_index).collect()
    }

    /// Build a manager from an explicit list of per-monitor configurations.
    pub fn from_config(configs: Vec<MonitorConfig>) -> Self {
        Self { monitors: configs }
    }

    /// Return the number of monitors managed by this instance.
    pub fn monitor_count(&self) -> usize {
        self.monitors.len()
    }

    /// Return the configuration for the monitor at `idx`, or `None` if out of
    /// range.
    pub fn config_for_monitor(&self, idx: usize) -> Option<&MonitorConfig> {
        self.monitors.get(idx)
    }

    /// Return an iterator over all monitor configurations.
    pub fn iter(&self) -> impl Iterator<Item = &MonitorConfig> {
        self.monitors.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_at_least_one_monitor() {
        let mgr = MultiMonitorManager::new();
        assert!(mgr.monitor_count() >= 1);
    }

    #[test]
    fn assign_surfaces_respects_count() {
        let cfgs = MultiMonitorManager::assign_surfaces(4);
        assert_eq!(cfgs.len(), 4);
        // All indices are distinct and in order.
        for (i, cfg) in cfgs.iter().enumerate() {
            assert_eq!(cfg.monitor_index, i);
        }
    }

    #[test]
    fn assign_surfaces_zero_gives_one() {
        let cfgs = MultiMonitorManager::assign_surfaces(0);
        assert_eq!(cfgs.len(), 1);
    }

    #[test]
    fn from_config_roundtrip() {
        let inputs = vec![
            MonitorConfig {
                monitor_index: 0,
                surface: "torus".into(),
                color_scheme: "cosmic".into(),
                geodesic_count: 20,
                speed: 1.5,
            },
            MonitorConfig {
                monitor_index: 1,
                surface: "sphere".into(),
                color_scheme: "ocean".into(),
                geodesic_count: 40,
                speed: 0.8,
            },
        ];
        let mgr = MultiMonitorManager::from_config(inputs);
        assert_eq!(mgr.monitor_count(), 2);
        assert_eq!(mgr.config_for_monitor(0).unwrap().surface, "torus");
        assert_eq!(mgr.config_for_monitor(1).unwrap().surface, "sphere");
        assert!(mgr.config_for_monitor(2).is_none());
    }

    #[test]
    fn surfaces_cycle_across_many_monitors() {
        let cfgs = MultiMonitorManager::assign_surfaces(25);
        // All geodesic_count defaults are 30.
        assert!(cfgs.iter().all(|c| c.geodesic_count == 30));
        // Surfaces are not all the same (they wrap around the cycle list).
        let surfaces: Vec<&str> = cfgs.iter().map(|c| c.surface.as_str()).collect();
        let first = surfaces[0];
        assert!(surfaces.iter().any(|s| *s != first));
    }
}
