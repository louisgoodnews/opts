//! Round trips of the session save: every section and a train formation.


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

#[test]
fn a_train_formation_survives_the_session_save() {
    use opts_core::formats::trains::{
        FormationSegment, FormationSource, Orientation, TrainFormation, VehicleRef,
    };
    let formation = TrainFormation {
        name: "Regio".into(),
        segments: vec![
            FormationSegment {
                vehicle: VehicleRef::File("vehicles\\BR218\\BR218_OR.ovh".into()),
                orientation: Orientation::Forward,
                powered: None,
            },
            FormationSegment {
                vehicle: VehicleRef::File("vehicles\\Bm238\\By438_mi.ovh".into()),
                orientation: Orientation::Backward,
                powered: Some(false),
            },
        ],
        source: FormationSource::ZugFile("Trains\\regio_grundorf.zug".into()),
    };
    let mut app = session_world();
    {
        let mut reg = app.world_mut().resource_mut::<SessionVehicles>();
        reg.request_spawn(SpawnParams {
            name: "Regio".into(),
            bus_file: "/nonexistent/BR218_OR.ovh".into(),
            formation: Some(formation.clone()),
            ..Default::default()
        });
        reg.request_spawn(SpawnParams {
            name: "Single".into(),
            bus_file: "/nonexistent/a.bus".into(),
            ..Default::default()
        });
    }
    let file = SaveFile::decode(&capture(&mut app).encode().expect("encode")).expect("decode");
    let mut other = session_world();
    restore(&mut other, &file);
    let records = other
        .world()
        .resource::<SessionVehicles>()
        .records()
        .to_vec();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].formation.as_ref(), Some(&formation));
    assert_eq!(records[1].formation, None, "a single vehicle stays single");
    assert_eq!(
        records[0].lifecycle,
        Lifecycle::Spawning,
        "the spawner builds the whole train"
    );
}
