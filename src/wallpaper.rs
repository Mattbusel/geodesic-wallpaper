//! Win32 WorkerW/Progman trick to parent our window behind desktop icons.
//! All unsafe is isolated here.

use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, BOOL};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SendMessageTimeoutW, EnumWindows,
    FindWindowExW, SetParent,
    SMTO_NORMAL,
};
use std::sync::OnceLock;

static WORKER_W: OnceLock<isize> = OnceLock::new();

fn hwnd_is_null(h: &HWND) -> bool {
    h.0 as isize == 0
}

/// Attempt to set the render window as a child of the WorkerW (behind desktop icons).
/// Returns true on success.
pub fn attach_to_desktop(hwnd: HWND) -> bool {
    unsafe {
        // Find Progman
        let progman = match FindWindowW(
            windows::core::w!("Progman"),
            None,
        ) {
            Ok(h) => h,
            Err(_) => {
                log::warn!("Could not find Progman window");
                return false;
            }
        };
        if hwnd_is_null(&progman) {
            log::warn!("Could not find Progman window");
            return false;
        }

        // Send magic message to spawn WorkerW
        let _ = SendMessageTimeoutW(
            progman,
            0x052C,
            WPARAM(0xD),
            LPARAM(0x1),
            SMTO_NORMAL,
            1000,
            None,
        );

        // Enumerate top-level windows to find WorkerW that has a SHELLDLL_DefView child
        let _ = EnumWindows(Some(find_worker_w), LPARAM(0));

        if let Some(&ww) = WORKER_W.get() {
            let worker = HWND(ww as _);
            match SetParent(hwnd, worker) {
                Ok(prev) if !hwnd_is_null(&prev) => {
                    log::info!("Attached to WorkerW successfully");
                    return true;
                }
                _ => {}
            }
        }
        log::warn!("WorkerW not found, falling back to normal window");
        false
    }
}

unsafe extern "system" fn find_worker_w(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    unsafe {
        // Look for a SHELLDLL_DefView child
        let shell_view = FindWindowExW(
            hwnd,
            None,
            windows::core::w!("SHELLDLL_DefView"),
            None,
        );
        let shell_ok = shell_view.as_ref().map(|h| !hwnd_is_null(h)).unwrap_or(false);
        if shell_ok {
            // The WorkerW we want is the next sibling of this window's parent
            let worker_w = FindWindowExW(
                None,
                hwnd,
                windows::core::w!("WorkerW"),
                None,
            );
            if let Ok(ww) = worker_w {
                if !hwnd_is_null(&ww) {
                    let _ = WORKER_W.set(ww.0 as isize);
                }
            }
        }
        BOOL(1) // continue enumeration
    }
}
