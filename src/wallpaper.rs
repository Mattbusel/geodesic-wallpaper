//! Windows desktop wallpaper window creation.
//! Uses HWND_BOTTOM + WM_WINDOWPOSCHANGING to stay pinned below all apps
//! but above the desktop background — reliable on Windows 10/11.
//! All unsafe isolated here.

use crate::events::KeyEvent;
use std::sync::OnceLock;
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, UpdateWindow, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, PostQuitMessage, RegisterClassExW, SetWindowPos, ShowWindow,
    CS_HREDRAW, CS_VREDRAW, HWND_BOTTOM, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WINDOWPOS,
    WM_DESTROY, WM_KEYDOWN, WM_WINDOWPOSCHANGING, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_POPUP, WS_VISIBLE,
};

/// Global sender for key events out of the window procedure.
static KEY_SENDER: OnceLock<std::sync::mpsc::Sender<KeyEvent>> = OnceLock::new();

/// Register the mpsc sender used to forward key events to the main loop.
pub fn set_key_sender(sender: std::sync::mpsc::Sender<KeyEvent>) {
    let _ = KEY_SENDER.set(sender);
}

fn hwnd_null(h: HWND) -> bool {
    h.0 as isize == 0
}

/// Create a borderless fullscreen window that stays below all other app windows.
/// Intercepts WM_WINDOWPOSCHANGING to prevent anything from raising it.
pub fn create_wallpaper_hwnd(width: i32, height: i32) -> Option<HWND> {
    unsafe {
        let hinstance = GetModuleHandleW(None).ok()?;
        let class_name = windows::core::w!("GeodesicWallpaper");
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassExW(&wc);

        let hwnd = match CreateWindowExW(
            WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
            class_name,
            windows::core::w!(""),
            WS_POPUP | WS_VISIBLE,
            0,
            0,
            width,
            height,
            None,
            None,
            hinstance,
            None,
        ) {
            Ok(h) => h,
            Err(e) => {
                tracing::error!("CreateWindowExW failed: {e}");
                return None;
            }
        };
        if hwnd_null(hwnd) {
            return None;
        }

        // Pin to bottom of Z-order — below all normal windows, above desktop
        let _ = SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, width, height, SWP_NOACTIVATE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = UpdateWindow(hwnd);

        tracing::info!("Wallpaper window created: {:?}", hwnd);
        Some(hwnd)
    }
}

/// Enumerate all connected monitors and return their bounding rectangles as
/// `(x, y, width, height)` in virtual-screen coordinates.
///
/// This uses the Win32 `EnumDisplayMonitors` API with a callback that collects
/// each monitor's `MONITORINFO.rcMonitor` rectangle.  Multi-monitor rendering
/// can then position wallpaper windows using these coordinates.
///
/// # Returns
///
/// A `Vec` with one entry per monitor.  An empty vec is returned if the
/// enumeration call fails entirely.
pub fn enumerate_monitors() -> Vec<(i32, i32, u32, u32)> {
    // Collect results through a raw pointer passed as lparam.
    let mut results: Vec<(i32, i32, u32, u32)> = Vec::new();

    unsafe extern "system" fn monitor_cb(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        unsafe {
            let vec = &mut *(lparam.0 as *mut Vec<(i32, i32, u32, u32)>);
            let mut info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            if GetMonitorInfoW(hmon, &mut info).as_bool() {
                let r = info.rcMonitor;
                let w = (r.right - r.left).max(0) as u32;
                let h = (r.bottom - r.top).max(0) as u32;
                vec.push((r.left, r.top, w, h));
            }
            BOOL(1) // continue enumeration
        }
    }

    unsafe {
        let ptr = &mut results as *mut Vec<(i32, i32, u32, u32)>;
        let _ = EnumDisplayMonitors(HDC::default(), None, Some(monitor_cb), LPARAM(ptr as isize));
    }

    results
}

/// Window procedure — intercepts WM_WINDOWPOSCHANGING to stay pinned at HWND_BOTTOM.
unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_DESTROY {
            PostQuitMessage(0);
            return LRESULT(0);
        }

        // Every time Windows tries to change our Z-order, force it back to BOTTOM.
        if msg == WM_WINDOWPOSCHANGING {
            let pos = &mut *(lp.0 as *mut WINDOWPOS);
            pos.hwndInsertAfter = HWND_BOTTOM;
            // Allow size/move changes but lock Z-order
            pos.flags |= SWP_NOZORDER;
            return LRESULT(0);
        }

        if msg == WM_KEYDOWN {
            let vk = wp.0 as u32;
            let event = match vk {
                0x5D | 0xDD => Some(KeyEvent::CycleSurface),     // ] — cycle forward
                0x5B | 0xDB => Some(KeyEvent::CycleSurfaceBack), // [ — cycle backward
                0x52 => Some(KeyEvent::ResetGeodesics),           // R
                0x46 => Some(KeyEvent::ToggleFpsHud),             // F
                0xBB | 0x6B => Some(KeyEvent::SpeedUp),           // + / numpad +
                0xBD | 0x6D => Some(KeyEvent::SpeedDown),         // - / numpad -
                0x20 => Some(KeyEvent::TogglePause),              // Space
                0x50 => Some(KeyEvent::Screenshot),               // P
                _ => None,
            };
            if let Some(ev) = event {
                if let Some(sender) = KEY_SENDER.get() {
                    let _ = sender.send(ev);
                }
            }
            return LRESULT(0);
        }

        DefWindowProcW(hwnd, msg, wp, lp)
    }
}
