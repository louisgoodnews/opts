//! Unit tests of `extract.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::vehicle::{classify_playable, is_ai_name, is_trailer_name};
use super::*;

#[test]
fn r11_silent_rail_is_not_playable() {
    assert!(!classify_playable(
        "bus",
        "NF6D_B_DT2.bus",
        false,
        false,
        true,
        true
    ));
    assert!(classify_playable(
        "bus",
        "NF6D_B_User.bus",
        false,
        false,
        true,
        false
    ));
}

#[test]
fn name_guards() {
    assert!(is_trailer_name("MAN_GN92_trail.bus") && is_trailer_name("x_heck.bus"));
    assert!(
        is_ai_name("AI_mb_w123.bus")
            && is_ai_name("MAN_SD84_KI.bus")
            && !is_ai_name("MAN_SD77.bus")
    );
}

#[test]
fn playability_rules() {
    assert!(classify_playable(
        "bus",
        "MAN_SD77.bus",
        false,
        false,
        true,
        false
    ));
    assert!(
        !classify_playable("ovh", "BX.ovh", false, false, false, false),
        "R1"
    );
    assert!(
        !classify_playable("bus", "x.bus", true, false, false, false),
        "R2"
    );
    assert!(
        !classify_playable("bus", "NF6D_B.bus", false, true, false, false),
        "R3"
    );
    assert!(
        classify_playable("bus", "NF6D_EEF_V.bus", false, true, true, false),
        "double-ended cab exempt"
    );
    assert!(
        !classify_playable("bus", "MAN_GN92_trail.bus", false, false, false, false),
        "R4"
    );
    assert!(
        !classify_playable("bus", "ai_golf.bus", false, false, true, false),
        "R5"
    );
    assert!(
        !classify_playable(
            "bus",
            "Vehicles/X/MB_KI_C2_E5_Solo.bus",
            false,
            false,
            true,
            false
        ),
        "R5: KI token in the middle"
    );
    assert!(
        !classify_playable(
            "bus",
            "Vehicles/Fahrrad/fahrrad_1.bus",
            false,
            false,
            false,
            false
        ),
        "R9: no driver workstation"
    );
    assert!(
        !classify_playable(
            "bus",
            "Vehicles/HOH_KI_Autos/Truck.bus",
            false,
            false,
            true,
            false
        ),
        "R10: AI folder"
    );
    assert!(
        classify_playable(
            "bus",
            "Vehicles/HOH_Kaiser_Bus/Bus.bus",
            false,
            false,
            true,
            false
        ),
        "'KI' inside a longer token is not an AI folder"
    );
}

#[test]
fn path_normalisation() {
    assert_eq!(
        under("Vehicles/X", "model\\..\\sound\\a.cfg"),
        "Vehicles/X/sound/a.cfg"
    );
}
