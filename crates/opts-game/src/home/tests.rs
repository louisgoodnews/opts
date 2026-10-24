//! Home: states, the Continue card and the drive request.

use super::*;
use crate::drive_history::DriveRecord;
use crate::shell::ShellPlugin;

fn record(map: &str, vehicle: &str, hour: u32) -> DriveRecord {
    DriveRecord {
        map: map.into(),
        vehicle: vehicle.into(),
        livery: "Beige".into(),
        weather_preset: "Clear".into(),
        hour,
        ..Default::default()
    }
}

fn home_app(history: &[DriveRecord]) -> App {
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
        .add_plugins(OptsGuiPlugin)
        .add_plugins(ShellPlugin)
        .add_plugins(HomePlugin);
    let mut drives = HomeDrives {
        persist: false,
        ..Default::default()
    };
    for r in history.iter().rev() {
        drives.history.record(r.clone());
    }
    app.insert_resource(drives);
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

#[test]
fn without_history_quick_drive_is_the_primary_action_and_has_focus() {
    let mut app = home_app(&[]);
    let ids = ids(&mut app);
    assert!(ids.iter().any(|i| i == "home.quick-drive"));
    assert!(!ids.iter().any(|i| i == "home.continue"));
    assert!(ids.iter().any(|i| i == "home.new-drive"));
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    assert_eq!(snap.focus.as_deref(), Some("home.quick-drive"));
}

#[test]
fn with_history_the_continue_card_has_one_drive_button_and_recents_follow() {
    let drives = [
        record("Berlin", "MAN SD77", 12),
        record("Grundorf", "MAN SD77", 8),
        record("Chicago", "Solaris", 20),
    ];
    let mut app = home_app(&drives);
    let ids = ids(&mut app);
    assert!(ids.iter().any(|i| i == "home.continue"));
    assert!(!ids.iter().any(|i| i == "home.quick-drive"));
    assert_eq!(
        ids.iter().filter(|i| i.starts_with("home.recent.")).count(),
        2
    );
    let snap = opts_gui::ui_tree::snapshot(app.world_mut());
    assert_eq!(snap.focus.as_deref(), Some("home.continue"));
}

#[test]
fn a_blocked_continue_does_nothing_and_an_enabled_one_requests_the_last_choices() {
    let mut app = home_app(&[record("Berlin", "MAN SD77", 12)]);
    let button =
        opts_gui::ui_tree::find_entity(app.world_mut(), "home.continue").expect("continue");
    // No catalogue in a test: the button is disabled and says why (the system that fills the
    // reason may need a frame more than the one that built the card).
    for _ in 0..3 {
        app.update();
        if app.world().get::<DisabledReason>(button).is_some() {
            break;
        }
    }
    assert!(app.world().get::<DisabledReason>(button).is_some());
    app.world_mut().write_message(Activated(button));
    for _ in 0..3 {
        app.update();
    }
    assert_ne!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame,
        "a disabled Continue must not start a drive"
    );
    app.world_mut().entity_mut(button).remove::<Disabled>();
    app.world_mut().write_message(Activated(button));
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::InGame
    );
    let setup = app.world().resource::<NewGameSetup>();
    assert_eq!(setup.selected_vehicle, "MAN SD77");
    assert_eq!(setup.hour, 12);
    assert_eq!(
        app.world()
            .resource::<HomeDrives>()
            .history
            .last()
            .map(|r| r.map.as_str()),
        Some("Berlin")
    );
}

#[test]
fn new_drive_opens_the_drive_destination_with_a_way_back() {
    let mut app = home_app(&[]);
    let button = opts_gui::ui_tree::find_entity(app.world_mut(), "home.new-drive").expect("new");
    app.world_mut().write_message(Activated(button));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(
        *app.world().resource::<State<AppState>>().get(),
        AppState::VehicleSelection
    );
    assert_eq!(
        app.world().resource::<crate::shell::BackStack>().0,
        vec!["home".to_owned()]
    );
}
