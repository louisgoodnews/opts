//! Main Menu visual validation and glassmorphic surface demonstration.
//!
//! # Usage
//! ```bash
//! # Run with solid surface fallback:
//! cargo run -p opts-game --example main_menu
//!
//! # Run with map-backed background image:
//! cargo run -p opts-game --example main_menu -- --map-path /path/to/omsi/maps/Berlin-Spandau
//! ```

use bevy::log::LogPlugin;
use bevy::prelude::*;
use opts_game::{GameSettings, OptsGamePlugin};
use std::path::PathBuf;

fn main() {
    // Parse optional CLI arguments: `--map-path <path>`
    let mut cli_map_path: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--map-path" {
            if let Some(path_str) = args.next() {
                cli_map_path = Some(PathBuf::from(path_str));
            }
        }
    }

    let mut settings = GameSettings::load_or_default();
    if let Some(path) = cli_map_path {
        settings.recent_map_path = Some(path);
    }

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "OPTS — Main Menu (Material Design 3 & Glassmorphic Surfaces)"
                            .to_string(),
                        resolution: (1280.0_f32, 800.0_f32).into(),
                        resize_constraints: bevy::window::WindowResizeConstraints {
                            min_width: 800.0,
                            min_height: 600.0,
                            ..default()
                        },
                        ..default()
                    }),
                    ..default()
                })
                .set(LogPlugin {
                    filter: "info,opts_game=debug,opts_gui=debug".into(),
                    level: bevy::log::Level::INFO,
                    ..default()
                }),
        )
        .insert_resource(settings)
        .add_plugins(OptsGamePlugin)
        .run();
}
