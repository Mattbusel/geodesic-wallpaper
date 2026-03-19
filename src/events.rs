//! Key events sent from the Win32 window procedure to the main loop.

/// Events produced by keyboard shortcuts in the wallpaper window.
#[derive(Debug, Clone, Copy)]
pub enum KeyEvent {
    /// Cycle to the next surface type.
    CycleSurface,
    /// Multiply rotation speed by 1.1.
    SpeedUp,
    /// Multiply rotation speed by 0.9.
    SpeedDown,
    /// Reinitialise all geodesics from the current config.
    ResetGeodesics,
    /// Toggle the FPS HUD overlay.
    ToggleFpsHud,
}
