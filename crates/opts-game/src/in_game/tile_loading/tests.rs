//! Unit tests of `tile_loading`.

use opts_hardware::fact::Fact;
use opts_hardware::recommend::MapStats;
use opts_hardware::HardwareProfile;

use super::*;

mod residency;

fn settings(load_all: Option<&str>, radius: Option<&str>) -> GameSettings {
    let mut s = GameSettings::default();
    if let Some(v) = load_all {
        s.set_raw(LOAD_ALL_KEY, v);
    }
    if let Some(v) = radius {
        s.set_raw(RADIUS_KEY, v);
    }
    s
}

fn machine(avail_gib: u64, cores: u32) -> HardwareProfile {
    let mut p = HardwareProfile::default();
    p.cpu.logical_cores = Fact::Known(cores);
    p.memory.total_mib = Fact::Known(avail_gib * 2048);
    p.memory.available_mib = Fact::Known(avail_gib * 1024);
    p
}

#[test]
fn the_defaults_are_off_and_automatic() {
    let choice = TileChoice::from_settings(&settings(None, None));
    assert_eq!(choice, TileChoice::default());
    assert!(!choice.load_all);
    assert!(choice.is_automatic());
}

#[test]
fn both_settings_are_read_and_a_garbled_radius_is_automatic() {
    let c = TileChoice::from_settings(&settings(Some("true"), Some("4")));
    assert_eq!(
        c,
        TileChoice {
            load_all: true,
            radius: 4
        }
    );
    assert!(TileChoice::from_settings(&settings(None, Some("many"))).is_automatic());
}

#[test]
fn load_all_wins_over_any_radius() {
    let c = TileChoice {
        load_all: true,
        radius: 2,
    };
    assert_eq!(c.mode(3, 8), TileMode::All);
}

#[test]
fn an_explicit_radius_is_kept_but_limited_to_the_map() {
    let pick = |radius| {
        TileChoice {
            load_all: false,
            radius,
        }
        .mode(3, 5)
    };
    assert_eq!(pick(2), TileMode::Radius(2));
    assert_eq!(pick(12), TileMode::Radius(5));
}

#[test]
fn automatic_uses_the_recommendation_within_the_map() {
    let auto = TileChoice::default();
    assert_eq!(auto.mode(3, 8), TileMode::Radius(3));
    assert_eq!(auto.mode(6, 2), TileMode::Radius(2));
    assert_eq!(auto.mode(0, 8), TileMode::Radius(1), "never below 1");
}

#[test]
fn the_default_radius_differs_between_two_hardware_profiles() {
    let map = MapStats::default();
    let weak = recommended(Some(&machine(2, 2)), &map);
    let strong = recommended(Some(&machine(48, 16)), &map);
    assert!(strong > weak, "{strong} <= {weak}");
    let auto = TileChoice::default();
    assert_ne!(auto.mode(weak, 8), auto.mode(strong, 8));
}

#[test]
fn resolve_applies_the_choice_to_this_machine_and_map() {
    let strong = machine(48, 16);
    let auto = TileChoice::default();
    let r = |profile: Option<&HardwareProfile>, map| resolve(auto, profile, map);
    assert!(matches!(r(Some(&strong), 8), TileMode::Radius(n) if n > 1));
    assert_eq!(
        r(Some(&strong), 1),
        TileMode::Radius(1),
        "a tiny map caps it"
    );
    assert_eq!(r(None, 8), TileMode::Radius(1));
    let all = TileChoice {
        load_all: true,
        radius: 0,
    };
    assert_eq!(resolve(all, None, 8), TileMode::All);
}

#[test]
fn the_streaming_budget_follows_the_setting() {
    let ms = |v: Option<&str>| {
        let mut s = GameSettings::default();
        if let Some(v) = v {
            s.set_raw(STREAM_BUDGET_KEY, v);
        }
        stream_budget_of(&s).as_millis()
    };
    assert_eq!(ms(None), DEFAULT_STREAM_BUDGET.as_millis());
    assert_eq!(ms(Some("16")), 16);
    assert_eq!(
        ms(Some("0")),
        0,
        "zero is the old one-tile-per-frame behaviour"
    );
    assert_eq!(ms(Some("500")), 50, "capped");
    assert_eq!(ms(Some("fast")), DEFAULT_STREAM_BUDGET.as_millis());
}

#[test]
fn without_a_profile_the_radius_is_the_old_3x3() {
    assert_eq!(recommended(None, &MapStats::default()), FALLBACK_RADIUS);
    assert_eq!(TileMode::default(), TileMode::Radius(1));
}

#[test]
fn the_map_radius_is_the_farthest_listed_tile_from_the_start() {
    let listed = [(0, 0), (1, 0), (-3, 2), (2, 2)];
    assert_eq!(max_radius(listed, (0, 0)), 3);
    assert_eq!(max_radius(listed, (-3, 2)), 5);
    assert_eq!(max_radius([], (0, 0)), 1);
}
