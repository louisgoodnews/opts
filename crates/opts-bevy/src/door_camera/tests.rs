//! Tests of the door camera: the doors of a cabin, the pose, the choice of the door.

use super::*;

/// Right front (entry), right front behind it (entry), right rear (exit), left rear (exit), in
/// Bevy axes (forward is -z).
fn doors() -> Vec<DoorPoint> {
    let door = |entry, index, x: f32, z: f32| DoorPoint {
        entry,
        index,
        position: Vec3::new(x, 0.4, z),
        side: if x < 0.0 { -1 } else { 1 },
        unit: None,
    };
    vec![
        door(true, 3, 1.25, -4.9),
        door(true, 0, 1.25, -4.2),
        door(false, 23, 1.25, 2.4),
        door(false, 21, -1.2, 2.5),
    ]
}

#[test]
fn zones_follow_the_position_along_the_vehicle() {
    let points = DoorPoints(doors());
    // z: -4.9, -4.2 (front), -2.4 and -2.5 are at the rear end of this door set.
    assert_eq!(points.zone(0), Zone::Front);
    assert_eq!(points.zone(1), Zone::Front);
    assert_eq!(points.zone(2), Zone::Rear);
    assert_eq!(points.zone(3), Zone::Rear);
    // Doors within a metre of each other have no front or rear.
    let near = DoorPoints(vec![doors()[0].clone(), doors()[1].clone()]);
    assert_eq!(near.zone(0), Zone::Middle);
    assert_eq!(points.zone(99), Zone::Middle);
}

#[test]
fn only_door_leaf_variables_count() {
    for yes in ["door_01", "c2_tuer_1_v_pos", "Door_2", "tür_pos"] {
        assert!(is_door_variable(yes), "{yes}");
    }
    for no in [
        "door_01_alpha",
        "door_warn",
        "doorspeed_0",
        "c2_cp_bt_tuer_1",
        "doortriggeropen_0",
        "c2_tuer_1_sound",
        "c2_tuer_2_open_req",
        "c2_tuer_1_h_stopped",
        "speed",
        "",
    ] {
        assert!(!is_door_variable(no), "{no}");
    }
}

#[test]
fn the_nearest_door_to_a_leaf_is_found_in_the_floor_plane() {
    let d: Vec<Vec3> = doors().iter().map(|p| p.position).collect();
    // A leaf at the right front (Bevy axes: forward is -z), a little above the floor and outside.
    assert_eq!(nearest_point(&d, Vec3::new(1.5, 1.2, -4.7)), Some(0));
    assert_eq!(nearest_point(&d, Vec3::new(1.5, 1.2, -4.25)), Some(1));
    // A leaf at the left rear.
    assert_eq!(nearest_point(&d, Vec3::new(-1.3, 1.0, 2.45)), Some(3));
    assert_eq!(nearest_point(&[], Vec3::ZERO), None);
}

#[test]
fn f4_cycles_through_the_doors_and_back_to_none_and_the_setting_gates_everything() {
    let mut s = DoorCamera::default();
    assert_eq!(s.shown(), None, "off by default");
    s.manual = Some(1);
    assert_eq!(
        s.shown(),
        None,
        "the setting is off: nothing is shown whatever was chosen"
    );
    s.enabled = true;
    s.manual = None;
    for want in [Some(0), Some(1), Some(2), None, Some(0)] {
        s.cycle(3);
        assert_eq!(s.manual, want);
    }
    s.manual = None;
    s.cycle(0);
    assert_eq!(s.manual, None, "no doors, nothing to choose");
    // The manual choice wins over the automatic one.
    s.auto = Some(2);
    assert_eq!(s.shown(), Some(2));
    s.manual = Some(1);
    assert_eq!(s.shown(), Some(1));
}

mod detection;
mod pictures;
mod pose;
