//! Unit tests for `super` (`sound.rs`).

use super::*;

const COCK: &str = "[sound]\ncock.WAV\n0.8\n[3d]\n0\n0\n0\n10\n[trigger]\ncock\n";

#[test]
fn a_triggered_one_shot_reads_completely() {
    let c = SoundConfig::parse(COCK);
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let d = &c.sounds[0];
    assert_eq!(
        (d.kind, d.file.as_str(), d.pitch),
        (SoundKind::Sound, "cock.WAV", 0.8)
    );
    assert_eq!((d.position, d.range), (Some([0.0, 0.0, 0.0]), 10.0));
    assert_eq!(d.triggers, vec!["cock".to_owned()]);
    assert!(!d.is_continuous());
}

#[test]
fn a_sound_without_modifiers_is_continuous() {
    let c = SoundConfig::parse("[sound]\ntrafohaus.wav\n1\n[3d]\n0\n0\n3\n25\n");
    assert!(c.sounds[0].is_continuous());
    let c =
        SoundConfig::parse("[sound]\ngarage.wav\n1\n[3d]\n0\n0\n0\n20\n[trigger]\nev\n[noloop]\n");
    assert!(!c.sounds[0].is_continuous() && c.sounds[0].noloop);
}

#[test]
fn volume_curves_condition_and_scenery_keywords() {
    let t = "[sound]\nlark.wav\n1\n[3d]\n1\n2\n3\n40\n\n[volcurve]\nSunAlt\n\n[pnt]\n-5\n0\n\n[pnt]\n10\n1\n\n[volcurve]\nDayOfYear\n[pnt]\n0\n0.5\n[pnt]\n100\n1\n[conditionSingle]\nTime\n43200\n1\n[onlyone]\n[checkloading]\n[viewpoint]\n2\n";
    let c = SoundConfig::parse(t);
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    let d = &c.sounds[0];
    assert_eq!(d.volcurves.len(), 2);
    assert_eq!(d.volcurves[0].variable, "SunAlt");
    assert!((d.volcurves[0].at(2.5) - 0.5).abs() < 1e-6);
    assert_eq!(d.volcurves[0].at(-50.0), 0.0);
    assert_eq!(d.volcurves[0].at(99.0), 1.0);
    assert_eq!(
        d.conditions,
        vec![Condition {
            variable: "Time".into(),
            value: 43200.0,
            mode: 1.0
        }]
    );
    assert!(d.only_one && d.check_loading);
    assert_eq!(d.viewpoint, Some(2));
    assert_eq!(d.position, Some([1.0, 2.0, 3.0]));
}

#[test]
fn prose_between_a_volcurve_and_its_points_is_a_comment() {
    let c = SoundConfig::parse(
        "[sound]\nlark.wav\n1\n[3d]\n0\n0\n0\n20\n[volcurve]\nDayOfYear\n\n\tvom 31.1. bis 31.7.\n\n[pnt]\n29.999\n0\n\n[pnt]\n30\n1\n",
    );
    assert!(c.warnings.is_empty(), "{:?}", c.warnings);
    assert_eq!(
        c.sounds[0].volcurves[0].points,
        vec![(29.999, 0.0), (30.0, 1.0)]
    );
}

#[test]
fn a_loop_sound_reads_its_five_lines() {
    let c = SoundConfig::parse("[loopsound]\nD_0826LUH@460.wav\n48000\nengine_n\n460\n1\n[3d]\n-0.7\n-2.8\n0.7\n3\n\n4.5\n");
    let d = &c.sounds[0];
    assert_eq!(d.kind, SoundKind::Loop);
    assert_eq!(
        (
            d.loop_rate,
            d.pitch_variable.as_deref(),
            d.pitch_reference,
            d.pitch_scale
        ),
        (Some(48000), Some("engine_n"), 460.0, 1.0)
    );
    assert_eq!(d.extra_distance, Some(4.5));
    assert!(d.is_continuous());
}

#[test]
fn comments_prose_and_commas_are_tolerated() {
    let t = "Hier wird gebrummt\n##########\n[sound]\nhum.wav\n1,0\nein Kommentar\n[3d]\n0,5\n0\n0\n12,5\n----\n";
    let c = SoundConfig::parse(t);
    assert_eq!(c.sounds[0].range, 12.5);
    assert_eq!(c.sounds[0].position, Some([0.5, 0.0, 0.0]));
}

#[test]
fn faults_become_warnings_not_failures() {
    let c = SoundConfig::parse("[3d]\n1\n2\n3\n4\n[wat]\n[pnt]\n1\n2\n[sound]\n");
    assert!(c.sounds.is_empty());
    assert!(c.warnings.len() >= 4, "{:?}", c.warnings);
    let c = SoundConfig::parse("[sound]\nx.wav\n1\n[volcurve]\nv\n[3d]\n1\n2\n");
    assert_eq!(c.sounds.len(), 1);
    assert!(c.sounds[0].volcurves.is_empty());
    assert_eq!(c.warnings.len(), 2, "{:?}", c.warnings);
}
