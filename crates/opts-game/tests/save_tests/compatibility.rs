//! Compatibility and preflight tests for session saves: v1 migration, foreign sections and missing content.

use super::*;

#[test]
fn an_old_version_fixture_migrates_and_unknown_sections_are_kept() {
    let text = include_str!("../fixtures/saves/v1-vehicles.save.json");
    let file = SaveFile::decode(text).expect("old fixture decodes");
    let app = session_world();
    let sections = app.world().resource::<SaveRegistry>().all().to_vec();
    let plan = plan_restore(&sections, &file);
    assert!(plan.warnings.is_empty(), "{:?}", plan.warnings);
    assert_eq!(plan.unknown, ["from_the_future"]);
    for absent in ["time", "weather", "camera", "counters", "simulation"] {
        assert!(
            plan.missing.iter().any(|m| m == absent),
            "{absent} takes its defaults"
        );
    }
    let (_, vehicles) = plan
        .steps
        .iter()
        .find(|(s, _)| s.id == "session_vehicles")
        .expect("migrated");
    assert_eq!(vehicles["version"], SAVE_SECTION_VERSION);
    let reg = SessionVehicles::from_section(vehicles).expect("reads");
    assert_eq!(reg.records().len(), 1);
    assert!(
        reg.records()[0].saved_state.is_none(),
        "version 1 has no live state"
    );

    // The unknown section survives an overwrite by this build.
    let store = SaveStore::new(dir("fixture"));
    store.write("old", &file).expect("write");
    let mut next = SaveFile::new(file.meta.clone());
    opts_game::save::registry::carry_over_foreign(&sections, &file, &mut next);
    store.write("old", &next).expect("overwrite");
    let back = store.read("old").expect("read").file;
    assert_eq!(
        back.sections["from_the_future"].data,
        json!({"anything": [1, 2, 3]})
    );
}

#[test]
fn missing_content_is_found_before_anything_is_applied() {
    let app = session_world();
    let sections = app.world().resource::<SaveRegistry>().all().to_vec();
    let file =
        SaveFile::decode(include_str!("../fixtures/saves/v1-vehicles.save.json")).expect("fixture");
    let root = dir("no-omsi");
    std::fs::create_dir_all(root.join("maps")).expect("maps");
    let issues = preflight(
        &sections,
        &file,
        &CheckEnv {
            root: Some(&root),
            fingerprint: &file.fingerprint,
        },
    );
    let map = issues
        .iter()
        .find(|i| i.kind == IssueKind::MissingMap)
        .expect("map");
    assert!(map.blocking);
    let vehicle = issues
        .iter()
        .find(|i| i.kind == IssueKind::MissingVehicle)
        .expect("vehicle");
    assert!(vehicle.blocking, "the only vehicle is missing");

    // With the map and one of two vehicles present the save still loads (with a warning).
    std::fs::create_dir_all(root.join("maps/Berlin-Spandau")).expect("map");
    std::fs::create_dir_all(root.join("Vehicles/A")).expect("vehicles");
    std::fs::write(root.join("Vehicles/A/bus_a.bus"), "x").expect("bus");
    let mut two = file.clone();
    two.sections
        .get_mut("session_vehicles")
        .expect("section")
        .data["vehicles"]
        .as_array_mut()
        .expect("array")
        .push(json!({
            "id": 2, "catalogue_id": "b", "name": "Bus B", "bus_file": "Vehicles/B/b.bus",
            "repaint": "", "setvars": [], "entry": "default", "spawn_entry_id": null,
            "spawn_entry_name": "", "spawn_point": null, "lifecycle": "alive",
            "despawn_reason": null, "error": null, "bus_rel": "Vehicles/B/b.bus"
        }));
    let issues = preflight(
        &sections,
        &two,
        &CheckEnv {
            root: Some(&root),
            fingerprint: &two.fingerprint,
        },
    );
    assert!(issues.iter().all(|i| !i.blocking), "{issues:?}");
    assert!(issues
        .iter()
        .any(|i| i.kind == IssueKind::MissingVehicle && i.subject == "Bus B"));
}
