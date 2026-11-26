//! The window size and the theme choices of the headless screen and HUD apps. Used by the screen
//! tests, the golden tests and the HUD matrix.

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};
use opts_gui::theme::mode::ThemeKind;
use opts_gui::theme::AccentPreset;

/// The user's theme choices that select `kind` and `accent` (the theme runtime turns them into
/// the `MaterialTheme`; inserting a `MaterialTheme` directly would be overwritten).
pub fn preferences(kind: ThemeKind, accent: AccentPreset) -> opts_gui::theme::ThemePreferences {
    use opts_gui::theme::{ThemeMode, ThemePreferences};
    ThemePreferences {
        mode: if kind == ThemeKind::Light {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        },
        high_contrast: kind == ThemeKind::HighContrast,
        accent,
        ..Default::default()
    }
}

/// Sets the primary window to `w` x `h` logical pixels.
pub fn resize(app: &mut App, w: u32, h: u32) {
    let mut q = app
        .world_mut()
        .query_filtered::<(Entity, &mut Window), With<PrimaryWindow>>();
    let mut ids = Vec::new();
    for (e, mut window) in q.iter_mut(app.world_mut()) {
        window.resolution = WindowResolution::new(w, h);
        ids.push(e);
    }
    // No window system runs headless, so the camera never learns its target size and every
    // percentage-sized root (the shell frame) would be 0 x 0: tell it directly.
    let mut cameras = app.world_mut().query::<&mut Camera>();
    for mut camera in cameras.iter_mut(app.world_mut()) {
        camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: UVec2::new(w, h),
            scale_factor: 1.0,
        });
    }
    for _ in 0..3 {
        app.update();
    }
}
