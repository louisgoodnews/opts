use super::*;

#[test]
fn every_section_round_trips() {
    let mut app = session_world();
    set_session_state(&mut app);
    let file = capture(&mut app);
    for id in [
        "session",
        "time",
        "weather",
        "session_vehicles",
        "active_vehicle",
        "camera",
        "counters",
        "simulation",
    ] {
        assert!(file.sections.contains_key(id), "{id} not captured");
    }
    // The file survives a trip through text.
    let file = SaveFile::decode(&file.encode().expect("encode")).expect("decode");

    // A different session: everything the sections own differs.
    let mut other = session_world();
    {
        let w = other.world_mut();
        w.resource_mut::<CameraAvailability>().cockpit = true;
        w.resource_mut::<InGameSessionInfo>().sim_elapsed_seconds = 1.0;
    }
    let plan = restore(&mut other, &file);
    assert!(plan.warnings.is_empty() && plan.unknown.is_empty());
    let w = other.world();
    assert_restored_session(w);
}
