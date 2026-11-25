//! The root plugin of the game: the settings and the theme at start-up, the core plugins, and the
//! camera and the state transitions the root systems drive.

use bevy::prelude::*;
use opts_gui::prelude::*;

use crate::app_state::AppState;
use crate::settings::GameSettings;
use crate::{
    action_settings, hardware, help, home, hud, in_game, look, map_profiles, menu, passengers,
    pause, player_sound, route_progress, save, scenery_sound, session_summary, settings_registry,
    settings_theme, shell, theme_files, transition_to_main_menu, trip_start, ui_typography,
    vehicle_keys, vehicle_sound, viewpoints, InGamePlugin, MenuCamera, NewGameMenuPlugin,
    SettingsScreenPlugin,
};

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
            .add_plugins(crate::driver_aids::DriverAidsPlugin)
            .add_plugins(route_progress::RouteProgressPlugin)
            .add_plugins(trip_start::TripStartPlugin)
            .add_plugins(in_game::weather::WeatherPlugin)
            .add_plugins(scenery_sound::SceneryAudioPlugin)
            .add_plugins(vehicle_sound::AiAudioPlugin)
            .add_plugins(player_sound::PlayerAudioPlugin)
            .add_plugins(opts_bevy::reflections::ReflectionsResearchPlugin)
            .add_plugins(opts_bevy::model::SeasonTexturePlugin)
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
