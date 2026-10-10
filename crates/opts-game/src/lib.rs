//! OPTS Game - Core application orchestration, state management, and consumer UI screens.

pub mod app;
pub mod app_state;
pub mod catalogue_service;
pub mod in_game;
pub mod menu;
pub mod new_game_screen;
pub mod omsi_root;
pub mod report_context;
pub mod settings;
pub mod settings_screen;
pub mod shutdown;
pub mod stub_screens;

use bevy::prelude::*;
use opts_gui::prelude::*;

pub use app::{build_app, AppArgs};
pub use app_state::AppState;
pub use in_game::{InGamePlugin, MenuCamera};
pub use menu::MainMenuPlugin;
pub use new_game_screen::{NewGameMenuPlugin, NewGameSetup};
pub use opts_cli::CliArgs;
pub use settings::GameSettings;
pub use settings_screen::SettingsScreenPlugin;
pub use shutdown::ShutdownPlugin;
pub use stub_screens::StubScreensPlugin;

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

        let mut initial_theme = if settings.is_dark_theme {
            MaterialTheme::dark()
        } else {
            MaterialTheme::light()
        };
        settings.apply_to_theme(&mut initial_theme);
        app.insert_resource(initial_theme);

        // Register core states and plugins
        app.init_state::<AppState>()
            .add_plugins(OptsGuiPlugin)
            .add_plugins(MainMenuPlugin)
            .add_plugins(SettingsScreenPlugin)
            .add_plugins(NewGameMenuPlugin)
            .add_plugins(StubScreensPlugin)
            .add_plugins(InGamePlugin);

        // Register global glass surface theme synchronization
        app.add_systems(Update, menu::glass::sync_glass_surfaces);

        // Transition from Loading to MainMenu once core plugins are online
        app.add_systems(Startup, setup_camera)
            .add_systems(OnEnter(AppState::Loading), transition_to_main_menu);
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MenuCamera));
}

fn transition_to_main_menu(
    mut next_state: ResMut<NextState<AppState>>,
    args: Option<Res<AppArgs>>,
) {
    if args.is_some_and(|a| a.0.autostart) {
        tracing::info!("--autostart: skipping menus and entering the in-game session");
        next_state.set(AppState::InGame);
        return;
    }
    tracing::info!("Core game systems ready; transitioning from Loading to MainMenu");
    next_state.set(AppState::MainMenu);
}
