//! Registered sections and damaged files.

use super::*;

fn noop_capture(_: &mut World) -> Result<Option<Value>, String> {
    Ok(Some(json!({ "lamps": 3 })))
}

fn counting_apply(world: &mut World, data: &Value) -> Result<(), String> {
    world.insert_resource(Lamps(data["lamps"].as_u64().unwrap_or(0) as u32));
    Ok(())
}

#[derive(Resource)]
pub(super) struct Lamps(u32);

#[test]
fn a_section_registered_in_a_test_needs_no_change_to_the_save_system() {
    let mut app = session_world();
    app.register_save_section(SaveSection {
        id: "test_lamps",
        version: 1,
        stage: Stage::World,
        order: 500,
        required: false,
        capture: noop_capture,
        prepare: None,
        apply: Some(counting_apply),
        migrate: None,
        check: None,
        about: "a section from a test",
    });
    let file = capture(&mut app);
    assert_eq!(file.sections["test_lamps"].data, json!({"lamps": 3}));
    let store = SaveStore::new(dir("independence"));
    store.write("s", &file).expect("write");
    let loaded = store.read("s").expect("read").file;
    let mut other = session_world();
    // Another build without the section keeps it as unknown.
    let plan = plan_restore(other.world().resource::<SaveRegistry>().all(), &loaded);
    assert_eq!(plan.unknown, ["test_lamps"]);
    other.register_save_section(
        *app.world()
            .resource::<SaveRegistry>()
            .get("test_lamps")
            .expect("registered"),
    );
    restore(&mut other, &loaded);
    assert_eq!(other.world().resource::<Lamps>().0, 3);
}

#[test]
fn damaged_and_newer_files_are_reported_not_loaded() {
    let store = SaveStore::new(dir("damaged"));
    std::fs::create_dir_all(store.dir()).expect("dir");
    std::fs::write(
        store.dir().join("bad.save.json"),
        "{\"magic\": \"OPTS-SAVE\", \"for",
    )
    .expect("write");
    std::fs::write(
        store.dir().join("new.save.json"),
        json!({"magic": "OPTS-SAVE", "format_version": 42}).to_string(),
    )
    .expect("write");
    let list = store.list();
    assert_eq!(list.len(), 2);
    let kinds: Vec<_> = list
        .iter()
        .map(|e| e.state.as_ref().err().map(opts_game::save::SaveError::kind))
        .collect();
    assert!(
        kinds.contains(&Some("corrupt")) && kinds.contains(&Some("newer_format")),
        "{kinds:?}"
    );
}
