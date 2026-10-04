//! Hot reload of config.toml through the debounced watcher.

use geodesic_wallpaper::hot_reload::{load_config_file, watch_config};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn write_cfg(path: &std::path::Path, n: usize) {
    std::fs::write(path, format!("num_geodesics = {n}\n")).unwrap();
}

/// Two saves 30 ms apart: the reload must end on the second one. The old
/// watcher reloaded on the first event and ignored events for 200 ms after,
/// so it could finish on the first save's contents.
#[test]
fn rapid_saves_end_on_latest_contents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write_cfg(&path, 10);

    let (tx, rx) = mpsc::channel();
    let _w = watch_config(&path, Duration::from_millis(200), move |res| {
        let _ = tx.send(res.map(|c| c.num_geodesics));
    })
    .unwrap();
    // Give the OS watcher a moment to register.
    std::thread::sleep(Duration::from_millis(200));

    write_cfg(&path, 20);
    std::thread::sleep(Duration::from_millis(30));
    write_cfg(&path, 30);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut last = None;
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(700)) {
            Ok(v) => last = Some(v),
            Err(mpsc::RecvTimeoutError::Timeout) if last.is_some() => break,
            Err(_) => {}
        }
    }
    assert_eq!(last, Some(Ok(30)));
}

/// A broken file is reported as an error, so the caller keeps its old config.
#[test]
fn broken_file_reports_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    write_cfg(&path, 10);

    let (tx, rx) = mpsc::channel();
    let _w = watch_config(&path, Duration::from_millis(100), move |res| {
        let _ = tx.send(res.is_ok());
    })
    .unwrap();
    std::thread::sleep(Duration::from_millis(200));
    std::fs::write(&path, "num_geodesics = [not toml").unwrap();
    let got = rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(!got);
}

#[test]
fn load_config_file_parses_and_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.toml");
    write_cfg(&path, 7);
    assert_eq!(load_config_file(&path).unwrap().num_geodesics, 7);
    assert!(load_config_file(&dir.path().join("missing.toml")).is_err());
}
