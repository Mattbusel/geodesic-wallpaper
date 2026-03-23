//! Windows system tray icon with a right-click context menu.
//!
//! Creates a message-only hidden window that hosts a Shell_NotifyIcon tray
//! entry. Right-clicking the tray icon shows a popup menu with controls for
//! pause/resume, surface switching, and quitting the application.
//!
//! Communication back to the main render loop is done through:
//! - [`TrayState::paused`] — an `AtomicBool` for pause/resume
//! - [`TrayState::surface_request`] — a `Mutex<Option<String>>` for surface changes

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, GetCursorPos,
    PostQuitMessage, RegisterClassExW, SetForegroundWindow, TrackPopupMenu, CS_HREDRAW, CS_VREDRAW,
    MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, WM_APP, WM_DESTROY, WNDCLASSEXW,
    WS_EX_NOACTIVATE, WS_OVERLAPPED,
};

/// Shared state exposed to the main render loop by the tray thread.
pub struct TrayState {
    /// `true` while the render loop should pause (skip stepping and drawing).
    pub paused: AtomicBool,
    /// Set to `Some("surface_name")` when the user picks a surface from the menu.
    pub surface_request: Mutex<Option<String>>,
    /// Set to `true` when the user picks Quit from the tray menu.
    pub quit_requested: AtomicBool,
}

impl TrayState {
    /// Create a new `TrayState` in the running/unpaused state.
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
            surface_request: Mutex::new(None),
            quit_requested: AtomicBool::new(false),
        }
    }

    /// Return `true` if the application is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Return and clear any pending surface switch request.
    pub fn take_surface_request(&self) -> Option<String> {
        self.surface_request.lock().ok()?.take()
    }

    /// Return `true` if the user has requested quit via the tray.
    pub fn quit_requested(&self) -> bool {
        self.quit_requested.load(Ordering::Relaxed)
    }

    /// Toggle the paused state and return the new value.
    pub fn toggle_pause(&self) -> bool {
        let was_paused = self.paused.load(Ordering::Relaxed);
        self.paused.store(!was_paused, Ordering::Relaxed);
        !was_paused
    }
}

impl Default for TrayState {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu item IDs for the tray popup.
const IDM_PAUSE: u32 = 1001;
const IDM_TORUS: u32 = 1010;
const IDM_SPHERE: u32 = 1011;
const IDM_SADDLE: u32 = 1012;
const IDM_ENNEPER: u32 = 1013;
const IDM_CATENOID: u32 = 1014;
const IDM_QUIT: u32 = 1020;

/// Message sent from the tray icon to our hidden window.
const WM_TRAYICON: u32 = WM_APP + 1;

/// Thread-local pointer to the `TrayState` used inside the window procedure.
///
/// We use a raw pointer because `SetWindowLongPtrW` / `GetWindowLongPtrW` is
/// the idiomatic Win32 approach, but that requires an unsafe cast anyway.
/// The pointer is valid for the lifetime of the tray thread.
static TRAY_STATE_PTR: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

/// Spawn a background thread that owns the tray icon and message loop.
///
/// Returns an [`Arc<TrayState>`] that can be polled from the main render loop.
pub fn spawn_tray(initial_surface: String) -> Arc<TrayState> {
    let state = Arc::new(TrayState::new());
    let state_clone = Arc::clone(&state);
    std::thread::spawn(move || {
        // Safety: raw pointer valid for lifetime of tray thread; never dereferenced
        // after the thread exits.
        let ptr = Arc::into_raw(state_clone) as usize;
        let _ = TRAY_STATE_PTR.set(ptr);
        unsafe { tray_thread(initial_surface, ptr) }
    });
    state
}

unsafe fn tray_thread(initial_surface: String, state_ptr: usize) {
    let hinstance = match GetModuleHandleW(None).ok() {
        Some(h) => h,
        None => {
            tracing::warn!("tray: GetModuleHandleW failed");
            return;
        }
    };

    let class_name = windows::core::w!("GeodesicTray");
    let wc = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(tray_wnd_proc),
        hInstance: hinstance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    RegisterClassExW(&wc);

    let hwnd = match CreateWindowExW(
        WS_EX_NOACTIVATE,
        class_name,
        windows::core::w!("GeodesicTray"),
        WS_OVERLAPPED,
        0,
        0,
        1,
        1,
        None,
        None,
        hinstance,
        None,
    ) {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!("tray: CreateWindowExW failed: {e}");
            return;
        }
    };

    // Register the tray icon.
    let mut tip = [0u16; 128];
    let tip_text = "Geodesic Wallpaper";
    for (i, c) in tip_text.encode_utf16().enumerate().take(127) {
        tip[i] = c;
    }
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAYICON,
        szTip: tip,
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);

    tracing::info!(surface = %initial_surface, "tray icon created");

    // Store the state ptr in a thread-local for the wnd_proc.
    let _ = state_ptr; // used via TRAY_STATE_PTR

    // Message loop for the tray window.
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, TranslateMessage, MSG,
    };
    let mut msg = MSG::default();
    loop {
        let r = GetMessageW(&mut msg, None, 0, 0);
        if r.0 == 0 || r.0 == -1 {
            break;
        }
        let _ = TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    // Clean up tray icon before thread exits.
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe extern "system" fn tray_wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_DESTROY {
            PostQuitMessage(0);
            return LRESULT(0);
        }

        if msg == WM_TRAYICON {
            // lp low-word = mouse message
            let mouse_msg = (lp.0 & 0xFFFF) as u32;
            const WM_RBUTTONUP: u32 = 0x0205;
            if mouse_msg == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            return LRESULT(0);
        }

        if msg == windows::Win32::UI::WindowsAndMessaging::WM_COMMAND {
            let cmd = (wp.0 & 0xFFFF) as u32;
            handle_menu_command(cmd);
            return LRESULT(0);
        }

        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let state_ptr = match TRAY_STATE_PTR.get() {
        Some(&p) => p,
        None => return,
    };
    let state = &*(state_ptr as *const TrayState);
    let is_paused = state.is_paused();

    let hmenu = match CreatePopupMenu() {
        Ok(m) => m,
        Err(_) => return,
    };

    let pause_label: Vec<u16> = if is_paused {
        "Resume\0".encode_utf16().collect()
    } else {
        "Pause\0".encode_utf16().collect()
    };
    let _ = AppendMenuW(
        hmenu,
        MF_STRING,
        IDM_PAUSE as usize,
        windows::core::PCWSTR(pause_label.as_ptr()),
    );
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());

    macro_rules! menu_item {
        ($label:expr, $id:expr) => {{
            let label: Vec<u16> = $label.encode_utf16().collect();
            let _ = AppendMenuW(
                hmenu,
                MF_STRING,
                $id as usize,
                windows::core::PCWSTR(label.as_ptr()),
            );
        }};
    }
    menu_item!("Surface: Torus\0", IDM_TORUS);
    menu_item!("Surface: Sphere\0", IDM_SPHERE);
    menu_item!("Surface: Saddle\0", IDM_SADDLE);
    menu_item!("Surface: Enneper\0", IDM_ENNEPER);
    menu_item!("Surface: Catenoid\0", IDM_CATENOID);
    let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, windows::core::PCWSTR::null());
    menu_item!("Quit\0", IDM_QUIT);

    let mut pt = windows::Win32::Foundation::POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        hmenu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN,
        pt.x,
        pt.y,
        0,
        hwnd,
        None,
    );
    let _ = DestroyMenu(hmenu);
}

fn handle_menu_command(cmd: u32) {
    let state_ptr = match TRAY_STATE_PTR.get() {
        Some(&p) => p,
        None => return,
    };
    let state = unsafe { &*(state_ptr as *const TrayState) };

    match cmd {
        IDM_PAUSE => {
            let was_paused = state.paused.fetch_xor(true, Ordering::Relaxed);
            tracing::info!(paused = !was_paused, "tray: pause toggled");
        }
        IDM_TORUS => set_surface(state, "torus"),
        IDM_SPHERE => set_surface(state, "sphere"),
        IDM_SADDLE => set_surface(state, "saddle"),
        IDM_ENNEPER => set_surface(state, "enneper"),
        IDM_CATENOID => set_surface(state, "catenoid"),
        IDM_QUIT => {
            tracing::info!("tray: quit requested");
            state.quit_requested.store(true, Ordering::Relaxed);
        }
        _ => {}
    }
}

fn set_surface(state: &TrayState, name: &str) {
    if let Ok(mut req) = state.surface_request.lock() {
        tracing::info!(surface = name, "tray: surface switch requested");
        *req = Some(name.to_owned());
    }
}
