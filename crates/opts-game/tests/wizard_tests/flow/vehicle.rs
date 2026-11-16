use super::*;

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
