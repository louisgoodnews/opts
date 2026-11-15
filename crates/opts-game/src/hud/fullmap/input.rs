//! The map's mouse and key input: drag, wheel zoom, fit and Home.

use super::*;

/// The logical size of the primary window, or the fallback.
fn screen_of(windows: &Query<&Window, With<PrimaryWindow>>) -> Vec2 {
    windows
        .single()
        .map_or(FALLBACK_SCREEN, |w| Vec2::new(w.width(), w.height()))
}

/// Drag pans, the wheel zooms toward the cursor, Home follows the vehicle again.
pub fn input(
    keyboard: Res<ButtonInput<KeyCode>>,
    overview: Res<OverviewRoads>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<FullMapState>,
) {
    let screen = screen_of(&windows);
    state.screen = screen;
    let cursor = windows.single().ok().and_then(Window::cursor_position);
    let mut changed = false;
    // Drag: the world follows the cursor.
    if mouse.pressed(MouseButton::Left) {
        if let Some(now) = cursor {
            if let Some(from) = state.drag_from {
                let delta = now - from;
                if delta != Vec2::ZERO {
                    state.viewport.pan_pixels(delta, screen);
                    state.follow = false;
                    changed = true;
                }
            }
            state.drag_from = Some(now);
        }
    } else {
        state.drag_from = None;
    }
    // Wheel: zoom toward the cursor (the middle of the screen without a cursor).
    for ev in wheel.read() {
        let lines = match ev.unit {
            MouseScrollUnit::Line => ev.y,
            MouseScrollUnit::Pixel => ev.y / 40.0,
        };
        if lines != 0.0 {
            let at = cursor.unwrap_or(screen / 2.0);
            state.viewport.zoom_at(at, screen, wheel_factor(lines));
            changed = true;
        }
    }
    if keyboard.just_pressed(KeyCode::End) || state.fit_request {
        state.fit_request = false;
        if let Some((low, high)) = overview.extent() {
            state.viewport = Viewport::fit(low, high, screen);
            state.follow = false;
            changed = true;
        }
    }
    if keyboard.just_pressed(KeyCode::Home) {
        state.follow = true;
        changed = true;
    }
    if changed {
        state.immediate = true;
    }
}
