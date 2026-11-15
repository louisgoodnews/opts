use super::*;

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
