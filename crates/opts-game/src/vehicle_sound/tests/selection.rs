//! Which voices and sounds of a configuration can be played.

use super::*;

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
