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
pub mod control_environment;
pub mod control_hotspots;
pub mod control_loading;
pub mod control_profiles;
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
pub mod theme_files;
pub mod ui_typography;
pub mod vehicle_keys;
pub mod vehicle_setup_settings;
pub mod vehicle_sound;
pub mod viewpoints;

use bevy::prelude::*;
use opts_gui::prelude::*;

pub use app::{build_app, AppArgs};
pub use app_state::AppState;
pub use in_game::{InGamePlugin, MenuCamera};
pub use new_game_screen::{NewGameMenuPlugin, NewGameSetup};
pub use opts_cli::CliArgs;
pub use settings::GameSettings;
pub use settings_screen::SettingsScreenPlugin;
pub use shutdown::ShutdownPlugin;

/// Root game plugin adding state machine, GUI engine, main menu, and stub screens.
pub struct OptsGamePlugin;

impl Plugin for OptsGamePlugin {
    fn build(&self, app: &mut App) {
        // Initialize persistent settings if not already present
        if !app.world().contains_resource::<GameSettings>() {
            app.insert_resource(GameSettings::load_or_default());
        }

        // Configure Material Theme based on settings
        let settings = app
            .world()
            .get_resource::<GameSettings>()
            .cloned()
            .unwrap_or_else(GameSettings::load_or_default);

        // The first theme comes straight from the settings (dark unless the user chose
        // otherwise); the `opts-gui` theme systems keep it in step afterwards.
        let mut prefs = settings.theme_preferences();
        prefs.custom = theme_files::load_theme_file(&settings.theme_file);
        let now = opts_gui::theme::ThemeClock::local().now();
        let mut initial_theme = MaterialTheme::with_scheme(prefs.scheme(None, now));
        let os_hint = settings_theme::os_reduced_motion_hint();
        initial_theme.motion.reduced_motion = settings.reduced_motion || os_hint;
        app.insert_resource(initial_theme)
            .insert_resource(prefs)
            .insert_resource(settings.ui_scale_percent())
            .insert_resource(settings.density)
            .insert_resource(opts_gui::theme::motion::MotionPreference {
                user_setting: settings.reduced_motion,
                os_hint,
            });

        // Register core states and plugins
        app.init_state::<AppState>()
            .add_plugins(OptsGuiPlugin)
            .add_plugins(settings_registry::SettingsRegistryPlugin)
            .add_plugins(map_profiles::MapProfilesPlugin)
            .add_plugins(hardware::HardwarePlugin)
            .add_plugins(vehicle_keys::VehicleKeysPlugin)
            .add_plugins(action_settings::ActionKeysPlugin)
            .add_plugins(shell::ShellPlugin)
            .add_plugins(home::HomePlugin)
            .add_plugins(help::HelpPlugin)
            .add_plugins(pause::PausePlugin)
            .add_plugins(save::SavePlugin)
            .add_plugins(hud::HudPlugin)
            .add_plugins(viewpoints::ViewpointPlugin)
            .add_plugins(session_summary::SessionSummaryPlugin)
            .add_plugins(SettingsScreenPlugin)
            .add_plugins(NewGameMenuPlugin)
            .add_plugins(look::LookPlugin)
            .add_plugins(ui_typography::UiTypographyPlugin)
            .add_plugins(opts_bevy::door_camera::DoorCameraPlugin)
            .add_plugins(passengers::PassengersPlugin)
            .add_plugins(route_progress::RouteProgressPlugin)
            .add_plugins(in_game::weather::WeatherPlugin)
            .add_plugins(scenery_sound::SceneryAudioPlugin)
            .add_plugins(vehicle_sound::AiAudioPlugin)
            .add_plugins(player_sound::PlayerAudioPlugin)
            .add_plugins(opts_bevy::reflections::ReflectionsResearchPlugin)
            .add_plugins(InGamePlugin);

        // Register global glass surface theme synchronization
        app.add_systems(Update, menu::glass::sync_glass_surfaces);

        // Settings -> theme, scale, density, motion; the desktop's colour scheme for the System mode;
        // no automatic theme switch while loading or driving
        app.add_systems(
            PreUpdate,
            (
                settings_theme::sync_theme_settings,
                settings_theme::poll_os_color_scheme,
                settings_theme::lock_theme_switching,
            )
                .before(opts_gui::theme::runtime::resolve_theme),
        );

        // Transition from Loading to MainMenu once core plugins are online
        app.add_systems(Startup, setup_camera)
            .add_systems(OnEnter(AppState::Loading), transition_to_main_menu);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, MenuCamera));
}

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
