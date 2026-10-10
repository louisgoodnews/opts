//! Developer aid: `--screenshot <FILE>` saves the primary window once the session is running.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
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
    mut commands: Commands,
    renderer: Option<Res<bevy::render::view::screenshot::CapturedScreenshots>>,
    window: Query<Entity, With<PrimaryWindow>>,
    mut exit: MessageWriter<AppExit>,
) {
    let Some(args) = args else { return };
    let Some(path) = args.0.screenshot.clone() else {
        return;
    };
    if !state.hud_spawned {
        return;
    }
    *waited += time.delta_secs();
    if *requested {
        // Give the asynchronous PNG writer a moment to flush.
        if *waited > args.0.screenshot_delay + 2.0 {
            exit.write(AppExit::Success);
        }
        return;
    }
    if *waited < args.0.screenshot_delay {
        return;
    }
    let (Some(_), Ok(window)) = (renderer, window.single()) else {
        tracing::error!("--screenshot needs a window; it cannot be used with --headless");
        exit.write(AppExit::Success);
        return;
    };
    commands
        .spawn(Screenshot::window(window))
        .observe(save_to_disk(path.clone()));
    tracing::info!(path = %path.display(), "screenshot requested");
    *requested = true;
}
