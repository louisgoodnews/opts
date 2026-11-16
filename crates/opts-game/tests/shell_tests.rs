//! The application shell: rail from the registry, availability, navigation, back stack, Esc.

use bevy::prelude::*;
use opts_game::shell::frame::{RailItem, ShellRoot};
use opts_game::shell::{
    Navigate, NavigateBack, NavigationRefused, PendingDeepLink, ShellContent, ShellPlugin,
};
use opts_game::{app_state::AppState, help::HelpPlugin, home::HomePlugin, settings::GameSettings};
use opts_gui::not_yet::NotAvailableYet;
use opts_gui::prelude::*;

fn shell_app() -> App {
    opts_game::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .insert_resource(opts_game::home::HomeDrives {
            persist: false,
            ..Default::default()
        })
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(HomePlugin)
        .add_plugins(HelpPlugin)
        .add_plugins(opts_game::settings_screen::SettingsScreenPlugin);
    app.update(); // start-up: the shell frame exists before any screen is built
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..4 {
        app.update();
    }
    app
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn go(app: &mut App, message: Navigate) {
    app.world_mut().write_message(message);
    for _ in 0..3 {
        app.update();
    }
}

fn back(app: &mut App) {
    app.world_mut().write_message(UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
}

fn refusals(app: &mut App) -> Vec<NavigationRefused> {
    let messages = app.world().resource::<Messages<NavigationRefused>>();
    messages.get_cursor().read(messages).cloned().collect()
}

#[path = "shell_tests/layout.rs"]
mod layout;
#[path = "shell_tests/navigation.rs"]
mod navigation;
#[path = "shell_tests/rail.rs"]
mod rail;
