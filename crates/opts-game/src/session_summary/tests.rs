//! The summary card.

use super::*;
use crate::pause::stats::SessionStats;
use crate::shell::ShellPlugin;

fn app_with(stats: SessionStats) -> App {
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
        .insert_resource(crate::home::HomeDrives {
            persist: false,
            ..Default::default()
        })
        .init_state::<ReportModalState>()
        .init_resource::<ReportModalData>()
        .init_resource::<NewGameSetup>()
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(crate::home::HomePlugin)
        .add_plugins(SessionSummaryPlugin);
    app.insert_resource(LastSession::new(&NewGameSetup::default(), stats));
    app.update();
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..4 {
        app.update();
    }
    app
}

fn ids(app: &mut App) -> Vec<String> {
    opts_gui::ui_tree::snapshot(app.world_mut())
        .flatten()
        .iter()
        .map(|n| n.id.clone())
        .collect()
}

fn stats(problems: usize) -> SessionStats {
    SessionStats {
        duration_s: 125.0,
        distance_m: 1500.0,
        max_speed_m_s: 12.0,
        errors_at_start: 0,
        errors_now: problems,
    }
}

#[test]
fn the_card_offers_the_three_ways_on_and_hides_a_report_without_problems() {
    let mut app = app_with(stats(0));
    let ids = ids(&mut app);
    for id in ["summary.again", "summary.change-vehicle", "summary.home"] {
        assert!(ids.contains(&id.to_owned()), "{id}: {ids:?}");
    }
    assert!(!ids.contains(&"summary.report".to_owned()));
}

#[test]
fn problems_logged_come_with_a_report_button() {
    let mut app = app_with(stats(3));
    assert!(ids(&mut app).contains(&"summary.report".to_owned()));
}

#[test]
fn drive_again_requests_the_same_choices_and_dismisses_the_card() {
    let mut app = app_with(stats(0));
    let button = opts_gui::ui_tree::find_entity(app.world_mut(), "summary.again").expect("again");
    app.world_mut().write_message(Activated(button));
    for _ in 0..3 {
        app.update();
    }
    assert!(app.world().get_resource::<LastSession>().is_none());
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame
    );
}

#[test]
fn home_dismisses_the_card_without_starting_anything() {
    let mut app = app_with(stats(0));
    let button = opts_gui::ui_tree::find_entity(app.world_mut(), "summary.home").expect("home");
    app.world_mut().write_message(Activated(button));
    for _ in 0..3 {
        app.update();
    }
    assert!(!ids(&mut app).contains(&"summary.home".to_owned()));
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::MainMenu
    );
}
