//! Pause overlay and leaving a drive.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

use super::*;
use crate::in_game::types::SimulationLoadState;
use crate::shell::ShellPlugin;
use opts_bevy::reporting::ReportModalData;

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
        .init_state::<ReportModalState>()
        .init_resource::<ReportModalData>()
        .init_resource::<VehicleMenu>()
        .init_resource::<SimulationLoadState>()
        .init_resource::<NewGameSetup>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(crate::settings_screen::SettingsScreenPlugin)
        .init_resource::<crate::in_game::types::InGameSessionInfo>()
        .init_resource::<crate::session_vehicles::SessionVehicles>()
        .init_resource::<opts_bevy::CameraState>()
        .init_resource::<opts_bevy::vehicle_remote::RemoteControls>()
        .add_plugins(PausePlugin)
        .add_plugins(crate::save::SavePlugin);
    let dir = std::env::var_os("HOME")
        .map_or_else(std::env::temp_dir, std::path::PathBuf::from)
        .join(".local/share/opts/benchmark/session-save/ui-tests")
        .join(format!("{:?}", std::thread::current().id()).replace(['(', ')'], ""));
    let _ = std::fs::remove_dir_all(&dir);
    app.insert_resource(crate::save::SaveStorage(Some(crate::save::SaveStore::new(
        dir,
    ))));
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app.world_mut()
        .resource_mut::<SimulationLoadState>()
        .is_complete = true;
    app
}

fn key(app: &mut App, code: KeyCode, logical: Key) {
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: logical.clone(),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    for _ in 0..3 {
        app.update();
    }
    app.world_mut().write_message(KeyboardInput {
        key_code: code,
        logical_key: logical,
        state: ButtonState::Released,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}

fn esc(app: &mut App) {
    key(app, KeyCode::Escape, Key::Escape);
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

fn click(app: &mut App, id: &str) {
    let entity = opts_gui::ui_tree::find_entity(app.world_mut(), id).expect(id);
    app.world_mut().write_message(Activated(entity));
    for _ in 0..4 {
        app.update();
    }
}

fn paused(app: &App) -> bool {
    app.world().resource::<Time<Virtual>>().is_paused()
}

mod environment_page;
mod leave;
mod save_list;
