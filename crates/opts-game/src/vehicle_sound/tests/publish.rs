//! The variables the sound model is fed, and the loops that depend on them.

use super::*;
use crate::vehicle_sound::stream::ai_loop;

#[test]
fn a_loop_is_silent_while_its_condition_does_not_hold() {
    let d = cfg("[loopsound]\nidle.wav\n44100\nengine_n\n650\n1\n[3d]\n0\n0\n0\n8\n[conditionSingle]\nvelocity\n3\n3\n")
            .sounds[0]
            .clone();
    let mut vars = AiVariables::default();
    publish_sound_variables(&mut vars, 0.0, 0.0, 0.0, 0.5, false, true);
    assert_eq!(
        ai_loop(&d, &vars).0,
        0.0,
        "standing: velocity is not above 3"
    );
    publish_sound_variables(&mut vars, 10.0, 0.0, 0.0, 0.5, false, true);
    assert!(ai_loop(&d, &vars).0 > 0.0);
}

#[test]
fn the_drive_state_variables_are_published() {
    let mut vars = AiVariables::default();
    publish_sound_variables(&mut vars, 0.0, 0.0, 0.0, 0.5, false, true);
    assert_eq!(vars.get("engine_on"), 1.0);
    assert_eq!(vars.get("antrieb_getr_aktugang"), 0.0);
    publish_sound_variables(&mut vars, 20.0 / 3.6, 0.0, 0.0, 0.5, true, true);
    assert_eq!(vars.get("antrieb_getr_aktugang"), 2.0);
    assert_eq!(vars.get("wiperrunning"), 1.0);
}

#[test]
fn variables_are_published_and_drive_level_and_pitch() {
    let mut vars = AiVariables::default();
    publish_sound_variables(&mut vars, 10.0, 0.6, 0.0, 0.5, false, true);
    assert!((vars.get("Velocity") - 36.0).abs() < 1e-4);
    assert!((vars.get("throttle") - 0.5).abs() < 1e-5);
    assert!(vars.get("wheel_rotationspeed_1_l") > 100.0);
    assert_eq!(vars.get("streetcond"), 0.0);
    let d = cfg("[loopsound]\nm.wav\n44100\nengine_n\n1000\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n0\n[pnt]\n72\n1\n")
            .sounds
            .remove(0);
    assert!((level_of(&d, &vars) - 0.5).abs() < 1e-4);
    let rpm = vars.get("engine_n");
    assert!((speed_of(&d, &vars) - rpm / 1000.0).abs() < 1e-4);
    // Standing: no throttle, idle.
    publish_sound_variables(&mut vars, 0.0, 1.0, 1.0, 0.5, true, true);
    assert_eq!(
        (
            vars.get("throttle"),
            vars.get("engine_n"),
            vars.get("streetcond")
        ),
        (0.0, 650.0, 1.0)
    );
}
