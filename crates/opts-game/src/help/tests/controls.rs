//! The controls table and the vehicle part of the page show the current keys.

use super::*;

#[test]
fn the_controls_table_shows_the_current_keys() {
    let mut app = app();
    let mut keys = opts_bevy::ActionKeys::default();
    keys.set("help", opts_bevy::KeyChord::parse("Ctrl+H").unwrap());
    app.insert_resource(keys);
    enter_help(&mut app);
    for _ in 0..3 {
        app.update();
    }
    let shown: Vec<String> = app
        .world_mut()
        .query_filtered::<&Text, With<crate::help::controls::ActionChip>>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(shown.iter().any(|t| t == "Ctrl+H"), "{shown:?}");
    assert!(!shown.iter().any(|t| t == "F1"), "{shown:?}");
    assert!(
        shown.iter().any(|t| t == "F6"),
        "the others keep theirs: {shown:?}"
    );
}

#[test]
fn the_vehicle_part_lists_the_current_vehicle_keys_and_the_cockpit_controls() {
    let mut app = app();
    app.world_mut()
        .resource_mut::<crate::settings::GameSettings>()
        .set_raw("controls.vehicle.lights", "Ctrl+G");
    let owner = app.world_mut().spawn(opts_bevy::PlayerVehicle).id();
    for name in ["cp_kluczyk_mov", "horn", "horn"] {
        app.world_mut().spawn(opts_bevy::MouseTrigger {
            owner,
            trigger: name.into(),
        });
    }
    enter_help(&mut app);
    for _ in 0..4 {
        app.update();
    }
    let texts = all_texts(&mut app);
    assert!(
        texts.iter().any(|t| t == "Ctrl+G"),
        "the rebound lights key: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t == "M"),
        "the default engine start key"
    );
    assert!(
        !texts.iter().any(|t| t == "L"),
        "the old lights key is gone"
    );
    assert_eq!(
        texts.iter().filter(|t| *t == "horn").count(),
        1,
        "a name once"
    );
    assert!(texts.iter().any(|t| t == "cp_kluczyk_mov"), "{texts:?}");
}
