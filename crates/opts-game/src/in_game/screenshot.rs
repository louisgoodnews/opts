//! Developer aid: `--screenshot <FILE>` saves the primary window once the session is running.

use bevy::prelude::*;
use bevy::render::view::screenshot::ScreenshotManager;
use bevy::window::PrimaryWindow;

use crate::in_game::types::SimulationLoadState;
use crate::AppArgs;

#[allow(clippy::too_many_arguments)]
/// Waits `--screenshot-delay` seconds after the HUD appears, saves the frame, then exits.
pub fn capture_screenshot_when_ready(
    args: Option<Res<AppArgs>>,
    state: Res<SimulationLoadState>,
    time: Res<Time>,
    mut waited: Local<f32>,
    mut requested: Local<bool>,
    mut manager: Option<ResMut<ScreenshotManager>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(args) = args else { return };
    let Some(path) = args.0.screenshot.clone() else {
        return;
    };
    if !state.hud_spawned {
        return;
    }
    *waited += time.delta_seconds();
    if *requested {
        // Give the asynchronous PNG writer a moment to flush.
        if *waited > args.0.screenshot_delay + 2.0 {
            exit.send(AppExit::Success);
        }
        return;
    }
    if *waited < args.0.screenshot_delay {
        return;
    }
    let (Some(manager), Ok(window)) = (manager.as_mut(), window.get_single()) else {
        tracing::error!("--screenshot needs a window; it cannot be used with --headless");
        exit.send(AppExit::Success);
        return;
    };
    match manager.save_screenshot_to_disk(window, &path) {
        Ok(()) => tracing::info!(path = %path.display(), "screenshot requested"),
        Err(e) => tracing::error!("screenshot failed: {e}"),
    }
    *requested = true;
}
