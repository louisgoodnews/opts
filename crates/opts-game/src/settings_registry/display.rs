//! Applies the display settings (window mode, window size, vertical sync) to the window.
//!
//! The page and the revert prompt never touch a window: they change settings and the
//! [`DisplayBackend`] applies them, so tests plug in a fake display.

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PresentMode, PrimaryWindow, VideoModeSelection, WindowMode};

use super::{SettingChanged, SettingsRegistry};
use crate::settings::GameSettings;

/// The keys the display backend handles.
pub const DISPLAY_KEYS: [&str; 3] = [
    "graphics.window_mode",
    "graphics.resolution",
    "graphics.vsync",
];

/// Something that can show a display setting.
pub trait DisplayApply: Send + Sync {
    /// Applies the stored text `value` of the display setting `key`.
    fn apply(&mut self, world: &mut World, key: &str, value: &str);
}

/// The display of the running game: the primary window.
pub struct WindowDisplay;

/// Parses `1920x1080`.
#[must_use]
pub fn parse_resolution(text: &str) -> Option<(u32, u32)> {
    let (w, h) = text.split_once('x')?;
    Some((w.trim().parse().ok()?, h.trim().parse().ok()?))
}

impl DisplayApply for WindowDisplay {
    fn apply(&mut self, world: &mut World, key: &str, value: &str) {
        let mut q = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
        let Ok(mut window) = q.single_mut(world) else {
            return;
        };
        match key {
            "graphics.window_mode" => {
                window.mode = match value {
                    "borderless" => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                    "fullscreen" => WindowMode::Fullscreen(
                        MonitorSelection::Current,
                        VideoModeSelection::Current,
                    ),
                    _ => WindowMode::Windowed,
                };
            }
            "graphics.resolution" => {
                if let Some((w, h)) = parse_resolution(value) {
                    window.resolution.set(w as f32, h as f32);
                }
            }
            "graphics.vsync" => {
                window.present_mode = if value == "true" {
                    PresentMode::AutoVsync
                } else {
                    PresentMode::AutoNoVsync
                };
            }
            _ => {}
        }
    }
}

/// The display that settings are applied to.
#[derive(Resource)]
pub struct DisplayBackend(pub Box<dyn DisplayApply>);

/// Applies every display change announced by [`SettingChanged`].
pub fn apply_display_changes(
    world: &mut World,
    mut cursor: Local<bevy::ecs::message::MessageCursor<SettingChanged>>,
) {
    let changes: Vec<SettingChanged> = {
        let Some(messages) = world.get_resource::<Messages<SettingChanged>>() else {
            return;
        };
        cursor
            .read(messages)
            .filter(|c| DISPLAY_KEYS.contains(&c.key.as_str()))
            .cloned()
            .collect()
    };
    if changes.is_empty() {
        return;
    }
    let Some(mut backend) = world.remove_resource::<DisplayBackend>() else {
        return;
    };
    for c in changes {
        backend.0.apply(world, &c.key, &c.new);
    }
    world.insert_resource(backend);
}

/// Whether the command line chose the window (size, mode or vsync) for this run.
#[must_use]
pub fn window_chosen_on_command_line(args: &opts_cli::CliArgs) -> bool {
    args.fullscreen || args.windowed || args.vsync || args.window_size != "1920x1080"
}

/// Applies the saved display settings to the window at start-up, unless the command line chose
/// the window (then the command line wins for this run).
pub fn apply_saved_display(world: &mut World) {
    if world
        .get_resource::<crate::app::AppArgs>()
        .is_some_and(|a| window_chosen_on_command_line(&a.0))
    {
        return;
    }
    let registry = world.resource::<SettingsRegistry>().clone();
    let settings = world.resource::<GameSettings>().clone();
    let Some(mut backend) = world.remove_resource::<DisplayBackend>() else {
        return;
    };
    for key in DISPLAY_KEYS {
        if settings.loaded_keys.contains(key) {
            if let Some(v) = registry.text(&settings, key) {
                backend.0.apply(world, key, &v);
            }
        }
    }
    world.insert_resource(backend);
}
