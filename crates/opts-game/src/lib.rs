//! OPTS Game - Core application orchestration, state management, and consumer UI screens.

pub mod action_settings;
pub mod app;
pub mod app_state;
pub mod audio_content;
pub mod bus_config;
pub mod bus_config_spawn;
pub mod catalogue_service;
pub mod content_facts;
pub mod control;
pub mod control_audio;
pub mod control_configs;
pub mod control_drive;
pub mod control_dynamics;
pub mod control_fuel;
pub mod control_environment;
pub mod control_hotspots;
pub mod control_loading;
pub mod control_profiles;
pub mod control_rail;
pub mod control_route;
pub mod control_save;
pub mod control_scheduled;
pub mod control_session;
pub mod control_setvars;
pub mod control_textures;
pub mod control_tiles;
pub mod control_trace;
pub mod control_ui;
pub mod drive_history;
pub mod entry;
pub mod first_run;
pub mod formation;
pub mod hardware;
pub mod help;
pub mod home;
pub mod hud;
pub mod in_game;
pub mod isolation;
pub mod language_packs;
pub mod look;
pub mod map_profiles;
pub mod menu;
pub mod new_game_screen;
pub mod omsi_root;
pub mod omsi_setup;
pub mod passengers;
pub mod pause;
pub mod player_sound;
pub mod report_context;
pub mod route_progress;
pub mod save;
pub mod scenery_sound;
pub mod session_summary;
pub mod session_vehicles;
pub mod settings;
pub mod settings_registry;
pub mod settings_screen;
pub mod settings_theme;
pub mod shell;
pub mod shutdown;
pub mod sound_logic;
pub mod start_options;
pub mod steam_library;
pub mod theme_files;
pub mod trip_start;
pub mod ui_typography;
pub mod vehicle_keys;
pub mod vehicle_fuel_settings;
pub mod vehicle_setup_settings;
pub mod vehicle_sound;
pub mod viewpoints;

mod plugin;

pub use plugin::OptsGamePlugin;

use bevy::prelude::*;

pub use app::{build_app, AppArgs};
pub use app_state::AppState;
pub use entry::run;
pub use in_game::{InGamePlugin, MenuCamera};
pub use new_game_screen::{NewGameMenuPlugin, NewGameSetup};
pub use opts_cli::CliArgs;
pub use settings::GameSettings;
pub use settings_screen::SettingsScreenPlugin;
pub use shutdown::ShutdownPlugin;

fn transition_to_main_menu(
    mut next_state: ResMut<NextState<AppState>>,
    args: Option<Res<AppArgs>>,
    setup: Option<ResMut<new_game_screen::NewGameSetup>>,
    weather: Option<Res<opts_bevy::WeatherCatalogue>>,
) {
    let (autostart, options) = args.as_deref().map_or((false, None), |a| {
        (
            a.0.autostart,
            Some(start_options::StartOptions::from_args(&a.0)),
        )
    });
    let mut setup = setup;
    if let (true, Some(options), Some(setup)) = (autostart, options, setup.as_mut()) {
        // The command line's choices of the drive (`--entry`, `--weather`, `--date`, `--time`,
        // `--setvar`); a wrong one is said and the default stands.
        match options.and_then(|o| o.apply(setup, weather.as_deref())) {
            Ok(()) => {}
            Err(e) => tracing::error!("start options: {e}"),
        }
    }
    if autostart {
        tracing::info!("--autostart: skipping menus and entering the in-game session");
        // Developer aid: `OPTS_DEV_HOUR=<0-23>` starts the session at that hour (night checks).
        if let (Some(setup), Some(h)) = (
            setup.as_mut(),
            std::env::var("OPTS_DEV_HOUR")
                .ok()
                .and_then(|h| h.parse::<u32>().ok()),
        ) {
            setup.hour = h.min(23);
            setup.minute = 0;
        }
        next_state.set(AppState::InGame);
        return;
    }
    tracing::info!("Core game systems ready; transitioning from Loading to MainMenu");
    next_state.set(AppState::MainMenu);
}
