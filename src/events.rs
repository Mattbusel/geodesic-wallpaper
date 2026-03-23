//! Key events sent from the Win32 window procedure to the main loop.

/// Events produced by keyboard shortcuts in the wallpaper window.
#[derive(Debug, Clone, Copy)]
pub enum KeyEvent {
    /// Cycle to the next surface type (`[` / `]` keys).
    CycleSurface,
    /// Cycle to the previous surface type (`[` key).
    CycleSurfaceBack,
    /// Multiply animation speed by 1.1 (`+` key).
    SpeedUp,
    /// Multiply animation speed by 0.9 (`-` key).
    SpeedDown,
    /// Reinitialise all geodesics from the current config (`R` key).
    ResetGeodesics,
    /// Toggle the FPS HUD overlay (`F` key).
    ToggleFpsHud,
    /// Toggle pause / resume (`Space` key).
    TogglePause,
    /// Capture the current frame and save it as a PNG (`P` key).
    Screenshot,

    // ── Parameter tuner ──────────────────────────────────────────────────
    /// Select the previous tunable parameter (`[` key).
    TunerPrevParam,
    /// Select the next tunable parameter (`]` key).
    TunerNextParam,
    /// Decrease the selected parameter by one step (`-` key).
    TunerDecrease,
    /// Increase the selected parameter by one step (`=` key).
    TunerIncrease,

    // ── Phase-portrait recorder ───────────────────────────────────────────
    /// Toggle phase-portrait recording on/off (`Shift+R` key).
    ToggleRecording,

    // ── Gallery mode ──────────────────────────────────────────────────────
    /// Toggle gallery mode on/off (`G` key).
    ToggleGallery,
    /// Skip to the previous surface in gallery mode (`LEFT` arrow key).
    GalleryPrev,
    /// Skip to the next surface in gallery mode (`RIGHT` arrow key).
    GalleryNext,
}
