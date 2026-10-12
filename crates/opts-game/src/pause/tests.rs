//! Pause overlay and leaving a drive.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;

use super::*;
use crate::shell::ShellPlugin;

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
        .add_plugins(PausePlugin);
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

#[test]
fn esc_in_a_drive_opens_the_overlay_pauses_and_never_leaves() {
    let mut app = app();
    esc(&mut app);
    assert!(app.world().resource::<Pause>().open);
    assert!(paused(&app));
    assert_eq!(state(&app), AppState::InGame);
    let ids = ids(&mut app);
    for id in [
        "resume", "vehicle", "settings", "controls", "report", "leave",
    ] {
        assert!(ids.contains(&format!("pause.{id}")), "{id}: {ids:?}");
    }
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    assert_eq!(
        snap.focus.as_deref(),
        Some("pause.resume"),
        "Resume has focus"
    );
}

#[test]
fn esc_again_and_resume_both_continue_the_drive() {
    let mut app = app();
    esc(&mut app);
    esc(&mut app);
    assert!(!app.world().resource::<Pause>().open);
    assert!(!paused(&app));
    esc(&mut app);
    click(&mut app, "pause.resume");
    assert!(!app.world().resource::<Pause>().open);
    assert!(!paused(&app));
    assert_eq!(state(&app), AppState::InGame);
}

#[test]
fn backspace_alone_does_not_pause_because_the_vehicle_uses_it() {
    let mut app = app();
    key(&mut app, KeyCode::Backspace, Key::Backspace);
    assert!(!app.world().resource::<Pause>().open);
}

#[test]
fn leaving_a_drive_that_moved_asks_and_cancel_keeps_the_session() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<SessionStats>()
        .step(10.0, 8.0);
    esc(&mut app);
    click(&mut app, "pause.leave");
    assert!(ids(&mut app).contains(&"dialog.pause.leave.home".to_owned()));
    assert!(ids(&mut app).contains(&"dialog.pause.leave.quit".to_owned()));
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    assert_eq!(snap.focus.as_deref(), Some("dialog.pause.leave.cancel"));
    click(&mut app, "dialog.pause.leave.cancel");
    assert_eq!(state(&app), AppState::InGame);
    assert!(app.world().resource::<Pause>().open);
}

#[test]
fn leaving_to_home_keeps_the_summary_and_resumes_time() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<SessionStats>()
        .step(10.0, 8.0);
    esc(&mut app);
    click(&mut app, "pause.leave");
    click(&mut app, "dialog.pause.leave.home");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(state(&app), AppState::MainMenu);
    let last = app.world().resource::<LastSession>();
    assert_eq!(last.stats.max_speed_m_s, 8.0);
    assert!(!paused(&app), "time runs again");
}

#[test]
fn an_unused_drive_also_asks_and_leaves_without_a_summary() {
    let mut app = app();
    esc(&mut app);
    click(&mut app, "pause.leave");
    assert_eq!(
        state(&app),
        AppState::InGame,
        "nothing happens before the answer"
    );
    click(&mut app, "dialog.pause.leave.home");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(state(&app), AppState::MainMenu);
    assert!(app.world().get_resource::<LastSession>().is_none());
}

#[test]
fn esc_cancels_the_leave_dialog_and_the_pause_overlay_returns() {
    let mut app = app();
    esc(&mut app);
    click(&mut app, "pause.leave");
    assert!(ids(&mut app).contains(&"dialog.pause.leave.cancel".to_owned()));
    esc(&mut app);
    assert!(!ids(&mut app).contains(&"dialog.pause.leave.cancel".to_owned()));
    assert_eq!(state(&app), AppState::InGame);
    assert!(
        ids(&mut app).contains(&"pause.resume".to_owned()),
        "overlay is back"
    );
}

#[test]
fn settings_in_the_pause_overlay_opens_the_page_and_esc_returns_to_the_pause_menu() {
    let mut app = app();
    esc(&mut app);
    click(&mut app, "pause.settings");
    let open = ids(&mut app);
    assert!(open.contains(&"settings/search".to_owned()), "{open:?}");
    assert!(!open.contains(&"pause.resume".to_owned()));
    assert_eq!(state(&app), AppState::InGame);
    esc(&mut app);
    let back = ids(&mut app);
    assert!(!back.contains(&"settings/search".to_owned()), "{back:?}");
    assert!(
        back.contains(&"pause.resume".to_owned()),
        "pause menu is back"
    );
    assert!(app.world().resource::<Pause>().open);
    assert!(paused(&app));
}

#[test]
fn esc_while_loading_cancels_the_load() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<SimulationLoadState>()
        .is_complete = false;
    esc(&mut app);
    assert_eq!(state(&app), AppState::MainMenu);
}
