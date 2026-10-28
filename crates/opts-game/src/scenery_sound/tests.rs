//! Unit tests for `super` (`scenery_sound.rs`).

use super::*;
use opts_core::formats::sound::SoundConfig;

fn host(time: f32, day_of_year: f32, sun: f32) -> HostValues {
    let mut h = HostValues::of(&SceneryClock::at(2026, 6, 1, f64::from(time)));
    h.time_s = time;
    h.day_of_year = day_of_year;
    h.sun_alt_deg = sun;
    h
}

fn lark() -> SoundDef {
    SoundConfig::parse(
        "[sound]\nlark.wav\n1\n[3d]\n0\n0\n0\n20\n[volcurve]\nDayOfYear\n[pnt]\n29.999\n0\n[pnt]\n30\n1\n[pnt]\n214\n1\n[pnt]\n214.1\n0\n[volcurve]\nSunAlt\n[pnt]\n-5\n0\n[pnt]\n10\n1\n",
    )
    .sounds
    .remove(0)
}

#[test]
fn the_level_follows_the_season_and_the_sun() {
    let d = lark();
    // Summer day, sun high: full. Winter: silent. Summer night: silent. Dawn: half.
    assert!((level_of(&d, &host(43200.0, 100.0, 40.0)) - 1.0).abs() < 1e-5);
    assert_eq!(level_of(&d, &host(43200.0, 10.0, 40.0)), 0.0);
    assert_eq!(level_of(&d, &host(0.0, 100.0, -30.0)), 0.0);
    assert!((level_of(&d, &host(21600.0, 100.0, 2.5)) - 0.5).abs() < 1e-5);
}

#[test]
fn a_voice_fades_from_its_range_to_six_times_that() {
    let v = SceneryVoice {
        def: Arc::new(lark()),
        file: None,
        playback: Playback::Loop,
        was_active: false,
    };
    let c = v.distance();
    assert_eq!((c.reference, c.max), (20.0, 120.0));
    assert_eq!(c.gain(10.0), 1.0);
    assert_eq!(c.gain(121.0), 0.0);
    // Tiny ranges still reach 40 m beyond.
    let mut d = lark();
    d.range = 2.0;
    let v = SceneryVoice {
        def: Arc::new(d),
        file: None,
        playback: Playback::Loop,
        was_active: false,
    };
    assert_eq!(v.distance().max, 42.0);
}

#[test]
fn unknown_variables_read_nothing_and_known_ones_the_clock() {
    let h = host(100.0, 5.0, 7.0);
    assert_eq!(host_variable("TIME", &h), Some(100.0));
    assert_eq!(host_variable("dayofyear", &h), Some(5.0));
    assert_eq!(host_variable("SunAlt", &h), Some(7.0));
    assert_eq!(host_variable("mystery", &h), None);
}
