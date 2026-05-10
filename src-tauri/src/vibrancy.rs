//! Native window vibrancy (macOS only).
//!
//! When enabled, the main window becomes a semi-transparent NSVisualEffect
//! view — the desktop wallpaper and other windows blur through the
//! claudedeck UI. On Windows/Linux this is a no-op for now (Windows has
//! Mica/acrylic via the same crate, but we'd need a different
//! `apply_*` call and platform-specific UX).

use tauri::Runtime;

#[tauri::command]
pub fn set_window_vibrancy<R: Runtime>(
    window: tauri::Window<R>,
    enabled: bool,
) -> Result<(), String> {
    apply(&window, enabled).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn apply<R: Runtime>(window: &tauri::Window<R>, enabled: bool) -> Result<(), String> {
    use window_vibrancy::{apply_vibrancy, clear_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

    if enabled {
        // `Sidebar` material reads as the macOS-standard window blur
        // — same effect Finder, Mail, and most native apps use.
        // FollowsWindowActiveState ties the blur intensity to whether
        // the window is focused, matching system convention.
        apply_vibrancy(
            window,
            NSVisualEffectMaterial::Sidebar,
            Some(NSVisualEffectState::FollowsWindowActiveState),
            None,
        )
        .map_err(|e| format!("apply vibrancy: {e}"))?;
    } else {
        clear_vibrancy(window).map_err(|e| format!("clear vibrancy: {e}"))?;
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn apply<R: Runtime>(_window: &tauri::Window<R>, _enabled: bool) -> Result<(), String> {
    // Quietly no-op outside macOS. The frontend still toggles the
    // `data-vibrancy` html attribute, which controls the CSS-side
    // semi-transparent panels — those work everywhere.
    Ok(())
}
