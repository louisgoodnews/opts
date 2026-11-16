//! UI rule tests for overlays and secondary screens: help, pause, first-run dialogs and summary.
use super::*;

// ---- The other shell screens and overlays -------------------------------------------------

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use opts_gui::testing::{act, focused};

fn base_app() -> App {
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
        .init_state::<opts_bevy::reporting::ReportModalState>()
        .init_resource::<opts_bevy::reporting::ReportModalData>()
        .init_resource::<opts_game::in_game::VehicleMenu>()
        .init_resource::<opts_game::in_game::types::SimulationLoadState>()
        .init_resource::<opts_game::new_game_screen::types::NewGameSetup>()
        .insert_resource(opts_game::home::HomeDrives {
            persist: false,
            ..Default::default()
        })
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(HomePlugin)
        .add_plugins(HelpPlugin)
        .add_plugins(opts_game::pause::PausePlugin)
        .add_plugins(opts_game::session_summary::SessionSummaryPlugin)
        .add_plugins(opts_game::first_run::FirstRunPlugin);
    app.update();
    app
}

fn go(app: &mut App, state: AppState) {
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(state);
    for _ in 0..4 {
        app.update();
    }
}

fn escape(app: &mut App) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: Key::Escape,
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

/// Every focusable element can be reached with `next` alone (modals trap focus, so this is the
/// whole traversal rule for an overlay; pressing Confirm would close it).
fn all_reachable(app: &mut App) -> Vec<String> {
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    let wanted: Vec<String> = snap
        .flatten()
        .iter()
        .filter(|n| n.focusable && n.state.visible && n.state.disabled.is_none())
        .map(|n| n.id.clone())
        .collect();
    let focusable = snap
        .flatten()
        .iter()
        .filter(|n| n.focusable && n.state.visible)
        .count();
    let mut reached = Vec::new();
    for _ in 0..focusable + 2 {
        act(app, UiAction::FocusNext);
        if let Some(e) = focused(app) {
            if let Some(id) = opts_gui::ui_tree::snapshot(app.world_mut())
                .focus
                .filter(|_| {
                    app.world()
                        .get::<opts_gui::interaction::state::Focused>(e)
                        .is_some()
                })
            {
                if !reached.contains(&id) {
                    reached.push(id);
                }
            }
        }
    }
    if std::env::var_os("OPTS_DEBUG_REACH").is_some() {
        eprintln!("reached {reached:?}\nwanted {wanted:?}");
    }
    wanted
        .into_iter()
        .filter(|w| !reached.contains(w))
        .collect()
}

fn hold_to_the_rules(app: &mut App, what: &str) {
    let w = app.world_mut();
    let small = rules::small_targets(w);
    assert!(small.is_empty(), "{what}: {}", small.join("\n"));
    let text = rules::small_text(app.world_mut());
    assert!(text.is_empty(), "{what}: {}", text.join("\n"));
    let reasons = rules::disabled_without_reason(app.world_mut());
    assert!(reasons.is_empty(), "{what}: {reasons:?}");
    let missed = all_reachable(app);
    assert!(missed.is_empty(), "{what}: never focused {missed:?}");
}

#[test]
fn the_help_page_holds_to_the_rules() {
    let mut app = base_app();
    go(&mut app, AppState::Help);
    hold_to_the_rules(&mut app, "help");
}

#[test]
fn the_pause_overlay_holds_to_the_rules() {
    let mut app = base_app();
    go(&mut app, AppState::InGame);
    app.world_mut()
        .resource_mut::<opts_game::in_game::types::SimulationLoadState>()
        .is_complete = true;
    escape(&mut app);
    hold_to_the_rules(&mut app, "pause overlay");
}

#[test]
fn the_first_run_dialogs_hold_to_the_rules() {
    use opts_game::first_run::{FirstRun, Step};
    for step in [Step::Choose, Step::Welcome] {
        let mut app = base_app();
        app.world_mut().resource_mut::<FirstRun>().persist = false;
        app.world_mut().resource_mut::<FirstRun>().step = Some(step.clone());
        go(&mut app, AppState::MainMenu);
        hold_to_the_rules(&mut app, &format!("first run {step:?}"));
    }
}

#[test]
fn the_session_summary_holds_to_the_rules() {
    let mut app = base_app();
    app.insert_resource(opts_game::session_summary::LastSession::new(
        &Default::default(),
        opts_game::pause::SessionStats {
            duration_s: 90.0,
            distance_m: 800.0,
            max_speed_m_s: 9.0,
            errors_at_start: 0,
            errors_now: 2,
        },
    ));
    go(&mut app, AppState::MainMenu);
    hold_to_the_rules(&mut app, "session summary");
}
