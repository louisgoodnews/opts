//! Tests of the sound logic: playback kinds, conditions, levels, speed and viewpoints.

use super::*;

use opts_core::formats::sound::SoundConfig;

fn def(text: &str) -> SoundDef {
    SoundConfig::parse(text).sounds.remove(0)
}

#[test]
fn keywords_decide_what_a_definition_does() {
    let d3 = "[3d]\n0\n0\n0\n5\n";
    assert_eq!(
        playback_of(&def(&format!("[sound]\nhum.wav\n1\n{d3}"))),
        Playback::Loop
    );
    assert_eq!(
        playback_of(&def(&format!(
            "[loopsound]\nm.wav\n44100\nengine_n\n600\n1\n{d3}"
        ))),
        Playback::Loop
    );
    assert_eq!(
        playback_of(&def(&format!("[sound]\nbell.wav\n1\n{d3}[trigger]\nev\n"))),
        Playback::OnTrigger
    );
    assert_eq!(
        playback_of(&def(&format!(
            "[sound]\nk.wav\n1\n{d3}[conditionSingle]\nfill\n1\n1\n[noloop]\n"
        ))),
        Playback::OnEdge
    );
    assert_eq!(
        playback_of(&def(&format!(
            "[sound]\nbeep.wav\n1\n{d3}[conditionSingle]\nalert\n1\n1\n"
        ))),
        Playback::Loop
    );
    assert_eq!(
        playback_of(&def(&format!(
            "[sound]\nhorn.wav\n1\n{d3}[conditionSingle]\nhupe\n3\n-1\n"
        ))),
        Playback::Unsupported
    );
    assert_eq!(
        playback_of(&def(&format!("[sound]\nx.wav\n1\n{d3}[noloop]\n"))),
        Playback::Unsupported
    );
}

#[test]
fn the_modes_compare_as_equal_below_and_above() {
    assert_eq!(compare(1.0, 1.0, 1.0), Some(true));
    assert_eq!(compare(1.0, 0.0, 1.0), Some(false));
    assert_eq!(compare(2.0, 0.5, 1.0), Some(true));
    assert_eq!(compare(2.0, 2.0, 1.0), Some(false));
    assert_eq!(compare(3.0, 700.0, 300.0), Some(true));
    assert_eq!(compare(3.0, 100.0, 300.0), Some(false));
    // 0 differs, 4 at least, 5 at most (❓ Q-SND-006: read from the variables they test).
    assert_eq!(
        compare(0.0, 0.5, 3.0),
        Some(true),
        "the horn volume is not 3"
    );
    assert_eq!(compare(0.0, 3.0, 3.0), Some(false));
    assert_eq!(compare(4.0, 2.0, 2.0), Some(true));
    assert_eq!(compare(4.0, 1.0, 2.0), Some(false));
    assert_eq!(compare(5.0, 2.0, 3.0), Some(true));
    assert_eq!(compare(5.0, 4.0, 3.0), Some(false));
    assert_eq!(compare(-1.0, 1.0, 1.0), None);
}

#[test]
fn all_conditions_must_hold() {
    let d = def("[sound]\ns.wav\n1\n[3d]\n0\n0\n0\n5\n[conditionSingle]\nengine_n\n300\n3\n[conditionSingle]\nvelocity\n1\n2\n");
    let read = |engine_n: f32, v: f32| {
        move |n: &str| match n {
            "engine_n" => engine_n,
            _ => v,
        }
    };
    assert_eq!(conditions_hold(&d.conditions, read(800.0, 0.5)), Some(true));
    assert_eq!(
        conditions_hold(&d.conditions, read(800.0, 5.0)),
        Some(false)
    );
    assert_eq!(
        conditions_hold(&d.conditions, read(100.0, 0.5)),
        Some(false)
    );
    assert_eq!(conditions_hold(&[], read(0.0, 0.0)), Some(true));
}

#[test]
fn level_and_speed_follow_the_variables() {
    let d = def("[loopsound]\nm.wav\n44100\nengine_n\n600\n1\n[3d]\n0\n0\n0\n5\n[volcurve]\nengine_n\n[pnt]\n400\n0\n[pnt]\n800\n1\n");
    let read = |n: &str| if n == "engine_n" { 600.0 } else { 0.0 };
    assert!((level(&d, read) - 0.5).abs() < 1e-5);
    assert!((speed(&d, read) - 1.0).abs() < 1e-5);
    let fast = |_: &str| 1200.0;
    assert!((speed(&d, fast) - 2.0).abs() < 1e-5);
}

#[test]
fn the_reference_value_is_the_natural_speed_whatever_the_scale() {
    // The Citybus 530's idle loop: reference 600, scale 2. At 598 rpm it plays at about its
    // natural speed, not at twice it; the scale makes the pitch follow the rpm more steeply.
    let d = def("[loopsound]\nm.wav\n44100\nengine_n\n600\n2\n[3d]\n0\n0\n0\n5\n");
    assert!((speed(&d, |_| 598.0) - 1.0).abs() < 0.01);
    assert!(
        (speed(&d, |_| 690.0) - 1.3).abs() < 1e-4,
        "15 % more rpm, 30 % higher"
    );
    // A turbo whine with the scale 0.05 hardly changes its pitch.
    let t = def("[loopsound]\nt.wav\n44100\nengine_turbo_RPM\n28\n0.05\n[3d]\n0\n0\n0\n5\n");
    assert!((speed(&t, |_| 56.0) - 1.05).abs() < 1e-4);
    // Far above the reference the speed is limited; at an engine speed of 0 the loop stops.
    assert_eq!(speed(&d, |_| 5000.0), 3.5);
    assert!(speed(&d, |_| 0.0) < MIN_SPEED);
    assert_eq!(loop_voice(&d, |_| 0.0).0, 0.0, "a stopped engine is silent");
    assert!(loop_voice(&d, |_| 600.0).0 >= 0.0 && loop_voice(&d, |_| 600.0).1 > 0.9);
    // A warning loop: pitch variable `brake_warning`, reference 1, no curve and no condition.
    let w = def("[loopsound]\nw.wav\n44100\nbrake_warning\n1\n1\n[3d]\n0\n0\n0\n5\n");
    assert_eq!(
        loop_voice(&w, |_| 0.0).0,
        0.0,
        "silent while nothing is wrong"
    );
    assert_eq!(
        loop_voice(&w, |_| 1.0),
        (1.0, 1.0),
        "sounding at its natural pitch when it is on"
    );
}

#[test]
fn viewpoints_split_inside_from_outside() {
    assert!(viewpoint_allows(None, true) && viewpoint_allows(None, false));
    // 2 and 6 are the interior sounds, 5 and 1 the exterior ones.
    assert!(viewpoint_allows(Some(2), true) && !viewpoint_allows(Some(2), false));
    assert!(viewpoint_allows(Some(6), true) && !viewpoint_allows(Some(6), false));
    assert!(viewpoint_allows(Some(5), false) && !viewpoint_allows(Some(5), true));
    assert!(viewpoint_allows(Some(1), false) && !viewpoint_allows(Some(1), true));
    assert!(viewpoint_allows(Some(7), true) && viewpoint_allows(Some(7), false));
}
