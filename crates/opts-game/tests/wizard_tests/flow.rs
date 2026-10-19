//! Walking the steps, choosing and driving.

use super::*;

#[test]
fn the_wizard_opens_on_where_with_the_map_list_and_the_summary() {
    let mut app = wizard("open");
    assert_eq!(step(&app), PlannerStep::Where);
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "Plan a drive"), "{t:?}");
    assert!(
        t.iter().any(|x| x.starts_with("Berlin-Spandau")),
        "summary names the map"
    );
    assert!(
        t.iter().any(|x| x.starts_with("Search 42 maps")),
        "the search field counts the maps: {:?}",
        t.iter()
            .filter(|x| x.contains("Search"))
            .collect::<Vec<_>>()
    );
    cleanup("open");
}

#[test]
fn next_back_skip_and_escape_walk_the_steps() {
    let mut app = wizard("walk");
    press(&mut app, "wizard.footer.next");
    assert_eq!(step(&app), PlannerStep::What);
    press(&mut app, "wizard.footer.next");
    assert_eq!(step(&app), PlannerStep::When);
    press(&mut app, "wizard.footer.back");
    assert_eq!(step(&app), PlannerStep::What);
    press(&mut app, "wizard.footer.skip");
    assert_eq!(step(&app), PlannerStep::Review);
    act(&mut app, UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(step(&app), PlannerStep::When, "Esc goes one step back");
    act(&mut app, UiAction::Back);
    act(&mut app, UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(step(&app), PlannerStep::Where);
    act(&mut app, UiAction::Back);
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(state(&app), AppState::MainMenu, "then out of the wizard");
    cleanup("walk");
}

#[test]
fn drive_starts_the_session_and_remembers_the_choices() {
    let mut app = wizard("drive");
    press(&mut app, "wizard.footer.skip");
    assert_eq!(step(&app), PlannerStep::Review);
    press(&mut app, "wizard.footer.drive");
    assert_eq!(state(&app), AppState::InGame);
    let saved = DriveHistory::load_from(&history_path("drive"));
    let setup = app.world().resource::<NewGameSetup>().clone();
    let last = saved.last().expect("the drive is in the history file");
    assert_eq!(last.map, setup.selected_map);
    assert_eq!(last.vehicle, setup.selected_vehicle);
    assert_eq!((last.hour, last.minute), (setup.hour, setup.minute));
    assert!(last.last_driven > 0);
    cleanup("drive");
}

pub(super) fn choose_in_what(app: &mut App, id: &str) {
    use opts_game::new_game_screen::step_where::StepList;
    let root = app
        .world_mut()
        .query::<(Entity, &StepList)>()
        .iter(app.world())
        .find(|(_, s)| s.0 == PlannerStep::What)
        .map(|(e, _)| e)
        .expect("the What list");
    app.world_mut()
        .write_message(opts_gui::patterns::master_detail::MasterDetailSelected {
            root,
            id: Some(id.to_owned()),
        });
    for _ in 0..4 {
        app.update();
    }
}

#[test]
fn drive_without_a_vehicle_is_a_clear_choice_in_the_what_step() {
    let mut app = wizard("none");
    press(&mut app, "wizard.footer.next");
    assert_eq!(step(&app), PlannerStep::What);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x == "Drive without a vehicle"),
        "a button says it: {t:?}"
    );
    assert!(!app.world().resource::<NewGameSetup>().no_vehicle);
    // The pinned list entry does the same as the button.
    choose_in_what(&mut app, ":none");
    assert!(app.world().resource::<NewGameSetup>().no_vehicle);
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x.contains("free camera")),
        "the detail says what happens: {t:?}"
    );
    app.world_mut().resource_mut::<NewGameSetup>().no_vehicle = false;
    choose_in_what(
        &mut app,
        &opts_game::catalogue_service::testing::fixture_lists().vehicles[0].id,
    );
    press(&mut app, "wizard.what.none");
    assert!(app.world().resource::<NewGameSetup>().no_vehicle);
    assert_eq!(step(&app), PlannerStep::When, "on to the next step");
    // Valid without a vehicle; the review says so; Drive starts and remembers it.
    press(&mut app, "wizard.footer.skip");
    assert_eq!(step(&app), PlannerStep::Review);
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x == "No vehicle"), "{t:?}");
    press(&mut app, "wizard.footer.drive");
    assert_eq!(state(&app), AppState::InGame);
    let saved = DriveHistory::load_from(&history_path("none"));
    assert!(saved.last().expect("history").no_vehicle);
    cleanup("none");
}

#[test]
fn choosing_a_vehicle_after_none_turns_the_map_only_start_off() {
    let mut app = wizard("none-off");
    press(&mut app, "wizard.footer.next");
    choose_in_what(&mut app, ":none");
    assert!(app.world().resource::<NewGameSetup>().no_vehicle);
    let id = opts_game::catalogue_service::testing::fixture_lists().vehicles[0]
        .id
        .clone();
    choose_in_what(&mut app, &id);
    let setup = app.world().resource::<NewGameSetup>().clone();
    assert!(!setup.no_vehicle);
    assert!(!setup.selected_vehicle.is_empty());
    cleanup("none-off");
}

#[test]
fn the_last_drive_is_applied_when_the_wizard_opens() {
    let tag = "remember";
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    let mut h = PlannerHistory::at(Some(history_path(tag)));
    h.history.record(DriveRecord {
        map: "Grundorf".into(),
        entry_point: "Bauernhof".into(),
        vehicle: "MAN SD200 SD77".into(),
        no_vehicle: false,
        livery: String::new(),
        weather_preset: "Rainy Afternoon".into(),
        year: 1975,
        month: 3,
        day: 4,
        hour: 18,
        minute: 0,
        setvars: Vec::new(),
        last_driven: 9,
    });
    let mut app = wizard_with(h);
    let s = app.world().resource::<NewGameSetup>().clone();
    assert_eq!(s.selected_map, "Grundorf");
    assert_eq!((s.year, s.hour), (1975, 18));
    assert!(
        app.world().resource::<PlannerState>().time_chosen,
        "a remembered drive brings its time: choosing a map keeps it"
    );
    assert_eq!(s.weather_preset, "Rainy Afternoon");
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x.contains("Grundorf")));
    cleanup(tag);
}

#[test]
fn an_invalid_choice_disables_next_with_a_reason() {
    let mut app = wizard("invalid");
    app.world_mut().resource_mut::<NewGameSetup>().selected_map = "Nowhere".into();
    for _ in 0..4 {
        app.update();
    }
    let next = find_entity(app.world_mut(), "wizard.footer.next").expect("next");
    let w = app.world();
    assert!(w
        .get::<opts_gui::interaction::state::Disabled>(next)
        .is_some());
    let reason = w
        .get::<opts_gui::interaction::state::DisabledReason>(next)
        .expect("a disabled control says why");
    assert_eq!(reason.0, "Choose a map first");
    press(&mut app, "wizard.footer.next");
    assert_eq!(
        step(&app),
        PlannerStep::Where,
        "a disabled Next does nothing"
    );
    cleanup("invalid");
}

/// P-1014: a wizard opened while the first scan still runs (empty lists) must show the lists once
/// the catalogue arrives, and leaving it afterwards must not touch despawned entities.
#[test]
fn a_wizard_opened_on_an_empty_catalogue_fills_when_the_lists_arrive() {
    let tag = "late";
    let _ = std::fs::remove_dir_all(history_path(tag).parent().expect("dir"));
    let fixture = opts_game::catalogue_service::testing::fixture_lists();
    let mut app = layout_app();
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
        .insert_resource(opts_game::new_game_screen::GameCatalogue::default())
        .insert_resource(PlannerHistory::at(Some(history_path(tag))))
        .insert_resource(ConfigLibrary::at(None))
        .add_plugins(NewGameMenuPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::VehicleSelection);
    for _ in 0..8 {
        app.update();
    }
    let before = texts(&mut app);
    assert!(
        !before.iter().any(|x| x.starts_with("Search 42 maps")),
        "empty catalogue: {before:?}"
    );
    *app.world_mut()
        .resource_mut::<opts_game::new_game_screen::types::GameCatalogue>() = fixture;
    for _ in 0..8 {
        app.update();
    }
    let after = texts(&mut app);
    assert!(
        after.iter().any(|x| x.starts_with("Search 42 maps")),
        "the list was built again: {after:?}"
    );
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    for _ in 0..8 {
        app.update();
    }
    cleanup(tag);
}
