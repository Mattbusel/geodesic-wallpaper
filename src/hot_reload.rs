//! Reload `config.toml` when it changes on disk.
//!
//! File events are debounced with [`notify_debouncer_mini`]: a burst of
//! events (an editor that truncates, writes, then renames a temp file into
//! place fires several) is collapsed into one reload that runs after the file
//! has been quiet for the debounce window. The file is read once, after the
//! last write, so the newest contents always win.
//!
//! The previous hand-rolled watcher reloaded on the *first* event and then
//! ignored every event for the next 200 ms, so a second save inside that
//! window was dropped and the wallpaper kept the older settings.
//!
//! ```no_run
//! use geodesic_wallpaper::hot_reload::watch_config;
//! use std::time::Duration;
//!
//! let _watcher = watch_config("config.toml".as_ref(), Duration::from_millis(250), |res| {
//!     match res {
//!         Ok(cfg) => println!("now {} geodesics", cfg.num_geodesics),
//!         Err(e) => eprintln!("config not reloaded: {e}"),
//!     }
//! })
//! .expect("could not watch config.toml");
//! // Keep `_watcher` alive for as long as you want reloads.
//! ```

use crate::config::Config;
use crate::error::GeodesicError;
use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, Debouncer};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Keeps the file watcher running. Reloads stop when this is dropped.
pub struct ConfigWatcher {
    _debouncer: Debouncer<RecommendedWatcher>,
    path: PathBuf,
}

impl ConfigWatcher {
    /// The config file being watched.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Read and parse `path`, applying the selected profile.
pub fn load_config_file(path: &Path) -> Result<Config, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read error: {e}"))?;
    let cfg: Config = toml::from_str(&text).map_err(|e| format!("parse error: {e}"))?;
    Ok(cfg.resolve_profile())
}

/// Watch `path` and call `on_change` with the freshly parsed config every
/// time it settles after a change.
///
/// The parent directory is watched (not the file itself) so that editors
/// which save by renaming a temp file over the original are still seen.
/// A file that fails to parse is reported as `Err` and the caller should keep
/// its current config.
pub fn watch_config<F>(
    path: &Path,
    debounce: Duration,
    mut on_change: F,
) -> Result<ConfigWatcher, GeodesicError>
where
    F: FnMut(Result<Config, String>) + Send + 'static,
{
    let path = std::path::absolute(path)
        .map_err(|e| GeodesicError::config(format!("bad config path: {e}")))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| GeodesicError::config("config path has no file name"))?
        .to_owned();
    let dir = path
        .parent()
        .ok_or_else(|| GeodesicError::config("config path has no parent folder"))?
        .to_path_buf();

    let target = path.clone();
    let mut debouncer = new_debouncer(debounce, move |res: DebounceEventResult| match res {
        Ok(events) => {
            if events
                .iter()
                .any(|e| e.path.file_name() == Some(file_name.as_os_str()))
            {
                on_change(load_config_file(&target));
            }
        }
        Err(e) => tracing::warn!("config watcher error: {e}"),
    })
    .map_err(|e| GeodesicError::config(format!("could not start config watcher: {e}")))?;

    debouncer
        .watcher()
        .watch(&dir, RecursiveMode::NonRecursive)
        .map_err(|e| GeodesicError::config(format!("could not watch {}: {e}", dir.display())))?;

    Ok(ConfigWatcher {
        _debouncer: debouncer,
        path,
    })
}
