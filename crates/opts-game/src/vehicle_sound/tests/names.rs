//! Telling AI vehicles from player ones by their file names.

use super::super::is_ai_name;

#[test]
fn ai_variants_are_told_by_name() {
    for yes in [
        "Vehicles/MAN_NL_NG/MAN_GN92_main_KI.bus",
        "sound\\sound_GN92_M_ai.cfg",
        "ai_peugeot_106_S1.ovh",
        "KI_bus.bus",
        "Vehicles/x/AI_C2_e5g_3door.bus",
    ] {
        assert!(is_ai_name(yes), "{yes}");
    }
    for no in [
        "MAN_GN92_main.bus",
        "sound\\sound_GN92_M.cfg",
        "Kiel.bus",
        "maintenance.cfg",
        "Vehicles/Rain/x.bus",
    ] {
        assert!(!is_ai_name(no), "{no}");
    }
}
