//! Help: tabs, F1, the overlay, the report keys.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

use super::*;
use opts_gui::prelude::*;

use crate::{pause::PausePlugin, shell::ShellPlugin};

fn app() -> App {
    crate::isolation::isolate_user_dirs();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(bevy::input::InputPlugin)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>()
        .insert_resource(crate::settings::GameSettings::default())
        .insert_resource(MaterialTheme::dark())
        .init_state::<AppState>()
        .init_resource::<crate::in_game::VehicleMenu>()
        .init_resource::<crate::in_game::types::SimulationLoadState>()
        .init_resource::<crate::new_game_screen::types::NewGameSetup>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(PausePlugin)
        .add_plugins(HelpPlugin);
    app.update();
    app
}

fn press(app: &mut App, code: KeyCode, logical: Key) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key: logical.clone(),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        for _ in 0..3 {
            app.update();
        }
    }
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn ids(app: &mut App) -> Vec<String> {
    opts_gui::ui_tree::snapshot(app.world_mut())
        .flatten()
        .iter()
        .map(|n| n.id.clone())
        .collect()
}

fn enter_help(app: &mut App) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Help);
    for _ in 0..4 {
        app.update();
    }
}

fn keys_shown(app: &mut App) -> Vec<&'static str> {
    app.world_mut()
        .query::<&opts_gui::l10n::L10nKey>()
        .iter(app.world())
        .map(|k| k.0)
        .collect()
}

fn all_texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect()
}

mod controls;
mod overlay;
mod page;
