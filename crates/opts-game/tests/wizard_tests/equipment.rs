//! The equipment dialog and saved configurations.

use super::flow::choose_in_what;
use super::*;

/// Opens the equipment dialog on a vehicle whose repaints carry a switch, a choice and a number.
fn open_equipment_dialog(app: &mut App) {
    use bevy::ecs::system::RunSystemOnce;
    use opts_game::new_game_screen::livery::LiveryState;
    use opts_game::new_game_screen::options::open_options;
    use opts_game::new_game_screen::repaints::VehicleRepaints;
    use opts_gui::l10n::Localization;

    let mut reps = VehicleRepaints {
        default_name: "Red".into(),
        ..VehicleRepaints::default()
    };
    reps.setvars.insert(
        "Red".into(),
        vec![
            ("vis_x".into(), "1".into()),
            ("display_ext".into(), "2".into()),
            ("vehicle_id".into(), "3964".into()),
        ],
    );
    reps.setvars
        .insert("Blue".into(), vec![("display_ext".into(), "5".into())]);
    app.world_mut().resource_mut::<LiveryState>().repaints = Some(reps);
    app.world_mut()
        .run_system_once(
            |mut commands: Commands,
             loc: Res<Localization>,
             catalogue: Res<opts_game::new_game_screen::GameCatalogue>,
             setup: Res<NewGameSetup>,
             liveries: Res<LiveryState>| {
                open_options(&mut commands, &loc, &catalogue, &setup, &liveries);
            },
        )
        .expect("open the dialog");
    for _ in 0..3 {
        app.update();
    }
}

/// Writes through the dialog's controls: a choice, a number and a switch.
fn exercise_option_controls(
    app: &mut App,
    (switch, combo, field): (Entity, Entity, Entity),
) {
    use opts_gui::interaction::events::{ComboboxSelected, SwitchToggled, TextChanged};
    // Choosing the second value (5) writes it; choosing the default again removes the override.
    app.world_mut().write_message(ComboboxSelected {
        entity: combo,
        selected_index: 1,
        selected_value: "5".into(),
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .get("display_ext"),
        Some(&5)
    );
    app.world_mut().write_message(ComboboxSelected {
        entity: combo,
        selected_index: 0,
        selected_value: "2".into(),
    });
    app.update();
    assert!(!app
        .world()
        .resource::<NewGameSetup>()
        .custom_setvars
        .contains_key("display_ext"));

    // A number: valid text is written, text that is no number or out of range is ignored.
    for (text, want) in [
        ("100", Some(100)),
        ("abc", Some(100)),
        ("-5", Some(100)),
        ("3964", None),
    ] {
        app.world_mut().write_message(TextChanged {
            entity: field,
            text: text.into(),
        });
        app.update();
        assert_eq!(
            app.world()
                .resource::<NewGameSetup>()
                .custom_setvars
                .get("vehicle_id")
                .copied(),
            want,
            "after typing {text:?}"
        );
    }

    // The switch writes 0 against its default 1.
    app.world_mut().write_message(SwitchToggled {
        entity: switch,
        is_on: false,
    });
    app.update();
    assert_eq!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .get("vis_x"),
        Some(&0)
    );
}

#[test]
fn the_equipment_dialog_offers_a_control_that_fits_each_option_and_writes_the_choices() {
    use opts_game::new_game_screen::options::{OptionChoice, OptionNumber, OptionSwitch};

    let mut app = wizard("options-dialog");
    open_equipment_dialog(&mut app);

    let switch = find_entity(app.world_mut(), "wizard.what.option.vis-x").expect("a switch");
    assert!(app.world().get::<OptionSwitch>(switch).is_some());
    let combo = find_entity(app.world_mut(), "wizard.what.option.display-ext").expect("a choice");
    let choice = app
        .world()
        .get::<OptionChoice>(combo)
        .expect("a choice box")
        .clone();
    assert_eq!(
        choice.values,
        vec![2, 5],
        "what the repaints give; 0 only when the repaint leaves it unset"
    );
    assert_eq!(choice.default, 2);
    let field = find_entity(app.world_mut(), "wizard.what.option.vehicle-id").expect("a number");
    assert!(app.world().get::<OptionNumber>(field).is_some());

    exercise_option_controls(&mut app, (switch, combo, field));
    cleanup("options-dialog");
}

fn config_library_path(tag: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!("opts_wizard_configs_{tag}_{}", std::process::id()))
        .join("bus_configurations.json")
}

fn wizard_with_configs(tag: &str) -> App {
    let path = config_library_path(tag);
    let _ = std::fs::remove_dir_all(path.parent().expect("dir"));
    wizard_with_library(PlannerHistory::at(None), ConfigLibrary::at(Some(path)))
}

#[test]
fn a_configuration_is_saved_from_the_plan_and_loads_back_after_another_choice() {
    let mut app = wizard_with_configs("save-load");
    press(&mut app, "wizard.footer.next");
    let vehicles = opts_game::catalogue_service::testing::fixture_lists().vehicles;
    assert!(vehicles.len() >= 2, "the fixture has two vehicles");
    choose_in_what(&mut app, &vehicles[0].id);
    app.world_mut()
        .resource_mut::<NewGameSetup>()
        .custom_setvars
        .insert("vis_x".into(), 0);
    let first_key = app
        .world()
        .resource::<NewGameSetup>()
        .selected_vehicle
        .clone();

    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x == "Configurations"),
        "the section is there: {t:?}"
    );
    press(&mut app, "wizard.what.configs.save");
    let lib = app.world().resource::<ConfigLibrary>();
    assert_eq!(lib.store.configurations.len(), 1);
    let saved = lib.store.configurations[0].clone();
    assert_eq!(saved.vehicle, vehicles[0].id);
    assert_eq!(saved.setvars, vec![("vis_x".to_string(), 0)]);
    // Written at once.
    let on_disk =
        opts_game::bus_config::BusConfigurations::load_from(&config_library_path("save-load"))
            .expect("file");
    assert_eq!(on_disk.configurations.len(), 1);

    // Another vehicle is chosen; loading the configuration brings the first back.
    choose_in_what(&mut app, &vehicles[1].id);
    assert_ne!(
        app.world().resource::<NewGameSetup>().selected_vehicle,
        first_key
    );
    press(&mut app, "wizard.what.configs.0.load");
    for _ in 0..8 {
        app.update();
    }
    let setup = app.world().resource::<NewGameSetup>().clone();
    assert_eq!(setup.selected_vehicle, first_key);
    assert!(!setup.no_vehicle);
    let used = app.world().resource::<ConfigLibrary>().store.configurations[0].last_used;
    assert!(used >= saved.last_used);
    let _ = std::fs::remove_dir_all(config_library_path("save-load").parent().expect("dir"));
}

#[test]
fn delete_asks_twice_star_and_duplicate_change_the_list() {
    let mut app = wizard_with_configs("manage");
    press(&mut app, "wizard.footer.next");
    let id = opts_game::catalogue_service::testing::fixture_lists().vehicles[0]
        .id
        .clone();
    choose_in_what(&mut app, &id);
    press(&mut app, "wizard.what.configs.save");
    press(&mut app, "wizard.what.configs.0.favourite");
    assert!(app.world().resource::<ConfigLibrary>().store.configurations[0].favourite);
    press(&mut app, "wizard.what.configs.0.duplicate");
    assert_eq!(
        app.world()
            .resource::<ConfigLibrary>()
            .store
            .configurations
            .len(),
        2
    );
    // The first press only asks, the second deletes.
    press(&mut app, "wizard.what.configs.1.delete");
    assert_eq!(
        app.world()
            .resource::<ConfigLibrary>()
            .store
            .configurations
            .len(),
        2
    );
    assert!(app
        .world()
        .resource::<ConfigLibrary>()
        .pending_delete
        .is_some());
    let t = texts(&mut app);
    assert!(
        t.iter().any(|x| x.starts_with("Delete? Press again")),
        "{t:?}"
    );
    press(&mut app, "wizard.what.configs.1.delete");
    assert_eq!(
        app.world()
            .resource::<ConfigLibrary>()
            .store
            .configurations
            .len(),
        1
    );
    assert!(app.world().resource::<ConfigLibrary>().store.configurations[0].favourite);
    let _ = std::fs::remove_dir_all(config_library_path("manage").parent().expect("dir"));
}

#[test]
fn a_configuration_of_a_vehicle_that_is_gone_says_so_and_does_not_load() {
    let mut app = wizard_with_configs("gone");
    press(&mut app, "wizard.footer.next");
    {
        let mut lib = app.world_mut().resource_mut::<ConfigLibrary>();
        lib.store.save(opts_game::bus_config::BusConfiguration::new(
            "Old bus",
            "no-such-vehicle",
            "",
            Vec::new(),
            1,
        ));
    }
    let id = opts_game::catalogue_service::testing::fixture_lists().vehicles[0]
        .id
        .clone();
    choose_in_what(&mut app, &id);
    let before = app.world().resource::<NewGameSetup>().clone();
    let t = texts(&mut app);
    assert!(t.iter().any(|x| x.contains("not installed")), "{t:?}");
    press(&mut app, "wizard.what.configs.0.load");
    assert_eq!(*app.world().resource::<NewGameSetup>(), before);
    let _ = std::fs::remove_dir_all(config_library_path("gone").parent().expect("dir"));
}
