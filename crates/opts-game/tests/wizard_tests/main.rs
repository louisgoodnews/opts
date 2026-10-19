//! Tests of the *Plan a drive* wizard in Bevy's real UI layout, on the fixture catalogue.

use bevy::prelude::*;
use opts_game::{
    app_state::AppState,
    drive_history::{DriveHistory, DriveRecord},
    new_game_screen::{
        configs::ConfigLibrary, FooterButton, NewGameMenuPlugin, NewGameSetup, PlannerHistory,
        PlannerState, PlannerStep,
    },
    settings::GameSettings,
};
use opts_gui::actions::UiAction;
use opts_gui::interaction::Activated;
use opts_gui::testing::{act, layout_app, rules};
use opts_gui::ui_tree::{find_entity, snapshot};

fn history_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!("opts_wizard_{tag}_{}", std::process::id()))
        .join("drive_history.json")
}

/// Pins the process's OMSI root to an empty installation, so the date lookup of the map step
/// (`global.cfg` of the map folder) never sees the host's real install: the fixture maps share
/// folder names with real maps. The first call wins for the whole test binary.
fn pin_fixture_root() {
    let root = std::env::temp_dir().join(format!("opts-wizard-root-{}", std::process::id()));
    std::fs::create_dir_all(root.join("maps")).expect("fixture root");
    let _ = opts_game::omsi_root::locate_omsi_root(Some(&root));
}

fn wizard_with(history: PlannerHistory) -> App {
    wizard_with_library(history, ConfigLibrary::at(None))
}

fn wizard_with_library(history: PlannerHistory, library: ConfigLibrary) -> App {
    pin_fixture_root();
    let mut app = layout_app();
    // The shell's content area: a fixed-size frame, like a 1280x720 window without the rail.
    let frame = app
        .world_mut()
        .spawn(Node {
            width: Val::Px(1280.0),
            height: Val::Px(720.0),
            ..default()
        })
        .id();
    app.insert_resource(opts_game::shell::ShellContent(Some(frame)));
    app.add_plugins(bevy::state::app::StatesPlugin)
        .insert_resource(GameSettings::default())
        .init_state::<AppState>()
        .insert_resource(opts_game::catalogue_service::testing::fixture_lists())
        .insert_resource(history)
        .insert_resource(library)
        .add_plugins(NewGameMenuPlugin);
    // The shell's one Esc mechanism; the shell's navigation is stood in for by a system that
    // answers `NavigateBack` with Home (the back stack is empty here).
    opts_game::shell::esc::build(&mut app);
    app.add_systems(
        Update,
        |mut back: MessageReader<opts_game::shell::nav::NavigateBack>,
         mut next: ResMut<NextState<AppState>>| {
            if back.read().count() > 0 {
                next.set(AppState::MainMenu);
            }
        },
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    for _ in 0..8 {
        app.update();
    }
    app
}

fn wizard(tag: &str) -> App {
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    wizard_with(PlannerHistory::at(Some(history_path(tag))))
}

fn step(app: &App) -> PlannerStep {
    app.world().resource::<PlannerState>().step
}

fn state(app: &App) -> AppState {
    *app.world().resource::<State<AppState>>().get()
}

fn press(app: &mut App, id: &str) {
    let e = find_entity(app.world_mut(), id).unwrap_or_else(|| panic!("no element {id}"));
    app.world_mut().write_message(Activated(e));
    for _ in 0..4 {
        app.update();
    }
}

fn texts(app: &mut App) -> Vec<String> {
    let mut q = app.world_mut().query::<&Text>();
    q.iter(app.world()).map(|t| t.0.clone()).collect()
}

fn cleanup(tag: &str) {
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
}

mod equipment;
mod flow;
mod plan;
mod step_rules;
