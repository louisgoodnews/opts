//! Unit tests of `vehicle_sound.rs`.

use super::*;
use crate::vehicle_sound::stream::ai_loop;

fn cfg(text: &str) -> SoundConfig {
    SoundConfig::parse(text)
}

#[test]
fn only_variables_the_traffic_model_states_are_supported() {
    for ok in [
        "Velocity",
        "THROTTLE",
        "engine_n",
        "Wheel_RotationSpeed_1_L",
        "StreetCond",
    ] {
        assert!(supported_variable(ok), "{ok}");
    }
    for bad in [
        "door_1",
        "in_door_sound_01",
        "engine_turbo_RPM",
        "Wheel_RotationSpeed_1",
        "-1",
        "m_wheel",
    ] {
        assert!(!supported_variable(bad), "{bad}");
    }
}

#[test]
fn a_configuration_is_cut_down_to_what_can_be_played() {
    let text = "[sound]\nrollen.wav\n3\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n0\n[pnt]\n80\n1\n\
[loopsound]\nmotor.wav\n44100\nengine_n\n631\n1\n[3d]\n0\n2\n0.5\n8\n[volcurve]\nThrottle\n[pnt]\n0\n0.3\n[pnt]\n1\n1\n\
[sound]\ndoor.wav\n1\n[3d]\n0\n0\n0\n5\n[trigger]\nev_door\n\
[sound]\nhiss.wav\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nin_door_sound_01\n[pnt]\n0\n1\n\
[loopsound]\nturbo.wav\n44100\nengine_turbo_RPM\n1000\n1\n[3d]\n0\n0\n0\n5\n\
[sound]\ngone.wav\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n1\n";
    let set = choose(&cfg(text), |n| (n != "gone.wav").then(|| PathBuf::from(n)));
    let files: Vec<_> = set.voices.iter().map(|(d, _)| d.file.as_str()).collect();
    assert_eq!(files, ["rollen.wav", "motor.wav"]);
    // The door and the turbo need the vehicle's scripts; only the missing file is lost.
    let scripted: Vec<_> = set.scripted.iter().map(|(d, _)| d.file.as_str()).collect();
    assert_eq!(scripted, ["door.wav", "hiss.wav", "turbo.wav"]);
    assert_eq!(set.skipped, 1);
}

#[test]
fn the_horn_is_a_host_event_and_conditions_on_the_drive_state_are_played() {
    let text = "[sound]\nhorn.wav\n1\n[3d]\n0\n3\n0\n30\n[trigger]\nev_AI_Horn\n\
[loopsound]\nidle.wav\n44100\nengine_n\n650\n1\n[3d]\n0\n0\n0\n8\n[conditionSingle]\nengine_on\n1\n1\n\
[loopsound]\nretarder.wav\n44100\nengine_n\n650\n1\n[3d]\n0\n0\n0\n8\n[conditionSingle]\nantrieb_retarderstufe\n2\n9\n\
[sound]\ncab.wav\n1\n[3d]\n0\n0\n0\n2\n[viewpoint]\n2\n[volcurve]\nVelocity\n[pnt]\n0\n1\n";
    let set = choose(&cfg(text), |n| Some(PathBuf::from(n)));
    let files = |v: &[(Arc<SoundDef>, PathBuf)]| -> Vec<String> {
        v.iter().map(|(d, _)| d.file.clone()).collect()
    };
    assert_eq!(files(&set.events), ["horn.wav"]);
    assert_eq!(files(&set.voices), ["idle.wav"]);
    assert!(set.scripted.is_empty());
    // A condition mode that is not understood, and a sound for inside the vehicle.
    assert_eq!(set.skipped, 2);
}

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
fn the_cap_keeps_the_first_voices() {
    let mut text = String::new();
    for i in 0..40 {
        text.push_str(&format!(
            "[sound]\nv{i}.wav\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nVelocity\n[pnt]\n0\n1\n"
        ));
    }
    let set = choose(&cfg(&text), |n| Some(PathBuf::from(n)));
    assert_eq!(set.voices.len(), MAX_AI_VOICES);
    assert_eq!(set.skipped, 40 - MAX_AI_VOICES);
}

#[test]
fn the_engine_speed_rises_drops_at_a_shift_and_rises_again() {
    assert_eq!(ai_engine_rpm(0.0, true), 650.0);
    assert!(ai_engine_rpm(6.0, true) > 650.0 && ai_engine_rpm(11.9, true) < 2100.1);
    let before = ai_engine_rpm(11.99, true);
    let after = ai_engine_rpm(12.1, true);
    assert!(after < before - 500.0, "{before} -> {after}");
    assert!(ai_engine_rpm(20.0, true) > after);
    assert!(ai_engine_rpm(300.0, true) <= 3200.0);
    let mut last = 0.0;
    for k in 0..=120 {
        let v = ai_engine_rpm(k as f32, true);
        assert!((600.0..=3200.0).contains(&v));
        last = v;
    }
    assert!(last > 650.0);
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

#[test]
fn a_car_idles_higher_and_revs_further_than_a_bus_and_the_gears_follow_the_speed() {
    assert_eq!(ai_engine_rpm(0.0, false), 800.0);
    assert!(ai_engine_rpm(15.0, false) > ai_engine_rpm(15.0, true));
    // Both shift: the speed drops at a gear change.
    for heavy in [true, false] {
        let before = ai_engine_rpm(if heavy { 11.99 } else { 17.99 }, heavy);
        let after = ai_engine_rpm(if heavy { 12.01 } else { 18.01 }, heavy);
        assert!(after < before - 400.0, "heavy {heavy}: {before} -> {after}");
    }
    assert_eq!(ai_gear(0.0, true), 0.0);
    assert_eq!(ai_gear(20.0, true), 2.0);
    assert_eq!(ai_gear(20.0, false), 2.0);
    assert_eq!(ai_gear(50.0, false), 3.0);
}

mod names {

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
}
