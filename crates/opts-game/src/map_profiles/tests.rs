//! Profile files, the fallback chain, validation, presets, the metrics ring, the advice rules and
//! the session overlay on `GameSettings`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use super::advice::{self, tiles_to_km2};
use super::profile::{chrono_id, settings_hash, slug, Dropped};
use super::*;
use crate::settings::GameSettings;
use crate::settings_registry::SettingsRegistry;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("opts-profiles-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn registry() -> SettingsRegistry {
    SettingsRegistry::with_builtin()
}

fn profile(name: &str, pairs: &[(&str, &str)]) -> MapProfile {
    let mut p = MapProfile::named(name);
    p.settings = pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect();
    p
}

#[test]
fn only_performance_and_simulation_keys_are_profile_eligible() {
    let r = registry();
    let keys = r.profile_keys();
    assert!(keys.len() >= 20, "{keys:?}");
    for must in [
        "graphics.water",
        "gameplay.traffic",
        "graphics.light_shafts",
    ] {
        assert!(keys.contains(&must), "{must}");
    }
    // Language, scale, key bindings and the display are never eligible.
    for never in [
        "interface.language",
        "ui_scale",
        "theme_mode",
        "controls.vehicle.horn",
        "graphics.resolution",
        "graphics.vsync",
        "omsi_path",
        "advanced.log_level",
    ] {
        assert!(!keys.contains(&never), "{never} must stay global");
    }
}

#[test]
fn presets_hold_only_valid_eligible_values() {
    let r = registry();
    let presets = presets();
    assert_eq!(presets.len(), 3);
    for p in presets {
        let (round, dropped) = MapProfile::parse(&p.to_toml(), &r).unwrap();
        assert!(dropped.is_empty(), "{}: {dropped:?}", p.name);
        assert_eq!(round.settings, p.settings, "{}", p.name);
    }
}

#[test]
fn loading_drops_unknown_global_and_invalid_keys_and_keeps_the_rest() {
    let text = r#"
schema = 1
name = "x"
[settings]
"gameplay.traffic" = "60"
"gameplay.pedestrians" = "9999"
"ui_scale" = "150"
"interface.language" = "fr"
"no.such.key" = "1"
"graphics.water" = "OFF"
"#;
    let (p, dropped) = MapProfile::parse(text, &registry()).unwrap();
    assert_eq!(
        p.settings.get("gameplay.traffic").map(String::as_str),
        Some("60")
    );
    assert_eq!(
        p.settings.get("graphics.water").map(String::as_str),
        Some("false")
    );
    assert_eq!(p.settings.len(), 2);
    let keys: Vec<&str> = dropped.iter().map(|d: &Dropped| d.key.as_str()).collect();
    for k in [
        "gameplay.pedestrians",
        "ui_scale",
        "interface.language",
        "no.such.key",
    ] {
        assert!(keys.contains(&k), "{k} not dropped: {keys:?}");
    }
    assert!(MapProfile::parse("this is = = not toml", &registry()).is_err());
}

#[test]
fn keys_are_filesystem_safe_and_chrono_ids_stable() {
    assert_eq!(slug("Berlin-Spandau"), "Berlin-Spandau");
    assert_eq!(slug("a/b"), "a%2Fb");
    assert_eq!(slug("../x"), "_..%2Fx");
    assert_eq!(chrono_id(&[]), "default");
    assert_eq!(
        chrono_id(&["1_FCA Heimspiele/02.11.25 - 14.00 Uhr"]),
        "1_FCA Heimspiele__02.11.25 - 14.00 Uhr"
    );
    assert_eq!(chrono_id(&["b", "a"]), chrono_id(&["a", "b"]));
    let long = "x".repeat(200);
    assert!(chrono_id(&[&long]).len() < 80);
    let key = MapKey::new("Ahlheim 5", &["0100_Neuer13N"]);
    assert_eq!(key.map_default().chrono_id, "default");
}

#[test]
fn the_fallback_chain_is_chrono_then_map_then_global() {
    let root = dir("chain");
    let store = ProfileStore::new(&root);
    let r = registry();
    let year = MapKey::new("Map A", &["2020"]);
    let base = year.map_default();

    // Nothing saved: the global settings apply.
    let none = store.resolve(&r, &year);
    assert_eq!((none.source, none.profile), (ProfileSource::Global, None));

    // The map's default serves a Chrono entry that has no file of its own.
    store
        .save(&base, &profile("base", &[("gameplay.traffic", "50")]))
        .unwrap();
    let got = store.resolve(&r, &year);
    assert_eq!(got.source, ProfileSource::Map);
    assert_eq!(got.profile.unwrap().name, "base");

    // The Chrono file wins once it exists.
    store
        .save(&year, &profile("y2020", &[("gameplay.traffic", "20")]))
        .unwrap();
    let got = store.resolve(&r, &year);
    assert_eq!(got.source, ProfileSource::Chrono);
    assert_eq!(got.profile.unwrap().settings["gameplay.traffic"], "20");
    // The default key itself never reports "Chrono".
    assert_eq!(store.resolve(&r, &base).source, ProfileSource::Map);

    // Deleting brings the chain back down.
    assert!(store.delete(&year).unwrap());
    assert!(!store.delete(&year).unwrap());
    assert_eq!(store.resolve(&r, &year).source, ProfileSource::Map);
    // The metrics and the static counts live in the same folder and are not profiles.
    std::fs::write(
        root.join("Map A").join("static.toml"),
        "tiles = 1\nbytes = 1\nobjects = 1\nsplines = 1\nai_pool = 1\n",
    )
    .unwrap();
    std::fs::write(root.join("Map A").join("metrics.toml"), "").unwrap();
    let listed: Vec<String> = store
        .list_map(&r, &year.map_id)
        .into_iter()
        .map(|(c, _)| c)
        .collect();
    assert_eq!(listed, vec!["default".to_string()]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_broken_file_is_moved_aside_and_the_chain_continues() {
    let root = dir("bad");
    let store = ProfileStore::new(&root);
    let r = registry();
    let key = MapKey::new("Map B", &[]);
    store.save(&key, &profile("ok", &[])).unwrap();
    let path = root.join("Map B").join("default.toml");
    std::fs::write(&path, "= = broken").unwrap();
    assert_eq!(store.resolve(&r, &key).source, ProfileSource::Global);
    assert!(root.join("Map B").join("default.toml.bad").exists());
    assert!(!path.exists());
    let _ = std::fs::remove_dir_all(&root);
}

fn session(chrono: &str, ms: f32, secs: f32) -> SessionMetrics {
    SessionMetrics {
        chrono: chrono.into(),
        avg_ms: ms,
        p95_ms: ms * 1.5,
        seconds: secs,
        vehicle: String::new(),
        settings_hash: settings_hash(&BTreeMap::new()),
        date: String::new(),
    }
}

#[test]
fn metrics_keep_five_sessions_per_chrono_key_and_skip_short_ones() {
    let root = dir("metrics");
    let store = ProfileStore::new(&root);
    assert!(!store
        .record_session("M", session("default", 20.0, 59.0))
        .unwrap());
    assert!(store.sessions("M").is_empty());
    for i in 0..8 {
        assert!(store
            .record_session("M", session("default", 10.0 + i as f32, 120.0))
            .unwrap());
    }
    store
        .record_session("M", session("2020", 40.0, 90.0))
        .unwrap();
    let all = store.sessions("M");
    let default: Vec<f32> = all
        .iter()
        .filter(|s| s.chrono == "default")
        .map(|s| s.avg_ms)
        .collect();
    assert_eq!(default, vec![13.0, 14.0, 15.0, 16.0, 17.0]);
    assert_eq!(all.iter().filter(|s| s.chrono == "2020").count(), 1);
    let key = MapKey {
        map_id: "M".into(),
        chrono_id: "default".into(),
    };
    assert_eq!(store.last_session(&key).unwrap().avg_ms, 17.0);
    let _ = std::fs::remove_dir_all(&root);
}

fn load(objects: usize, pool: usize) -> advice::MapLoadProfile {
    advice::MapLoadProfile {
        tile_count: 100,
        object_count: objects,
        spline_count: 1000,
        ai_pool_vehicles: pool,
        map_size_km2: tiles_to_km2(100),
        ..Default::default()
    }
}

fn ids(p: &advice::MapLoadProfile, traffic: i64) -> Vec<AdviceId> {
    advise(p, traffic).iter().map(|a| a.id).collect()
}

#[test]
fn the_heuristics_follow_the_documented_tiers() {
    assert!((tiles_to_km2(100) - 9.0).abs() < 1e-4);
    assert!(ids(&load(1_095, 10), 100).contains(&AdviceId::LightMap));
    assert!(!ids(&load(41_000, 10), 100).contains(&AdviceId::GeometryAboveTypical));
    assert!(ids(&load(60_000, 10), 100).contains(&AdviceId::GeometryAboveTypical));
    let heavy = advise(&load(300_000, 10), 100);
    let high = heavy
        .iter()
        .find(|a| a.id == AdviceId::GeometryHigh)
        .unwrap();
    assert_eq!(high.preset, Some("performance"));
    assert!(high
        .suggested
        .iter()
        .any(|(k, _)| k == "gameplay.tile_radius"));
    // Every suggested key is a map-profile key with a valid value.
    let r = registry();
    for a in advise(&load(300_000, 300), 150) {
        for (k, v) in a.suggested {
            let d = r.get(&k).unwrap_or_else(|| panic!("{k} unknown"));
            assert_eq!(
                d.profile,
                crate::settings_registry::ProfileScope::Map,
                "{k}"
            );
            crate::settings_registry::normalize(d, &v).unwrap_or_else(|e| panic!("{k}: {e}"));
        }
    }
    // The AI advice needs a large pool and a high traffic setting.
    assert!(ids(&load(10_000, 250), 150).contains(&AdviceId::HighAiLoad));
    assert!(!ids(&load(10_000, 250), 80).contains(&AdviceId::HighAiLoad));
    assert!(!ids(&load(10_000, 100), 150).contains(&AdviceId::HighAiLoad));
}

#[test]
fn frame_time_advice_uses_the_last_session() {
    let mut p = load(30_000, 10);
    assert!(ids(&p, 100).contains(&AdviceId::NotPlayedYet));
    p.avg_frame_time_ms = Some(40.0);
    assert!(ids(&p, 100).contains(&AdviceId::RanSlowly));
    assert!(!ids(&p, 100).contains(&AdviceId::NotPlayedYet));
    p.avg_frame_time_ms = Some(12.0);
    assert!(ids(&p, 100).contains(&AdviceId::RanSmoothly));
    p.avg_frame_time_ms = Some(25.0);
    let v = ids(&p, 100);
    assert!(!v.contains(&AdviceId::RanSlowly) && !v.contains(&AdviceId::RanSmoothly));
}

#[test]
fn a_session_overlay_never_reaches_the_file_and_restores_exactly() {
    let root = dir("overlay");
    std::fs::create_dir_all(&root).unwrap();
    let mut s = GameSettings::default();
    s.db_path_override = Some(root.join("settings.db"));
    s.set_raw("gameplay.traffic", "100");
    s.set_raw("texture_filtering", "16");
    s.save().unwrap();
    let before = s.clone();

    let mut values = BTreeMap::new();
    values.insert("gameplay.traffic".to_string(), "20".to_string());
    values.insert("texture_filtering".to_string(), "4".to_string());
    values.insert("graphics.bloom".to_string(), "true".to_string());
    s.overlay_session(&values);
    assert_eq!(s.raw("gameplay.traffic").as_deref(), Some("20"));
    assert_eq!(s.raw("graphics.bloom").as_deref(), Some("true"));
    assert_eq!(s.overlaid_keys().len(), 3);

    // Saving while the profile is on writes the global values.
    s.set_raw("ui_scale", "125");
    s.save().unwrap();
    let mut on_disk = GameSettings::default();
    let conn = rusqlite::Connection::open(root.join("settings.db")).unwrap();
    on_disk.apply_from_connection(&conn).unwrap();
    assert_eq!(on_disk.raw("gameplay.traffic").as_deref(), Some("100"));
    assert_eq!(on_disk.raw("texture_filtering").as_deref(), Some("16"));
    assert_eq!(on_disk.raw("graphics.bloom"), None);
    assert_eq!(
        on_disk.raw("ui_scale").as_deref(),
        Some("125"),
        "other changes still save"
    );

    // Restoring brings back the global values, including "nothing stored".
    s.restore_session();
    assert_eq!(s.raw("gameplay.traffic").as_deref(), Some("100"));
    assert_eq!(s.raw("texture_filtering").as_deref(), Some("16"));
    assert_eq!(s.raw("graphics.bloom"), None);
    assert!(s.overlaid_keys().is_empty());
    assert_eq!(s.raw("ui_scale").as_deref(), Some("125"));
    let _ = before;
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn overlaying_again_restores_the_earlier_overlay_first() {
    let mut s = GameSettings::default();
    s.set_raw("gameplay.traffic", "100");
    let mut a = BTreeMap::new();
    a.insert("gameplay.traffic".to_string(), "20".to_string());
    s.overlay_session(&a);
    let mut b = BTreeMap::new();
    b.insert("gameplay.pedestrians".to_string(), "5".to_string());
    s.overlay_session(&b);
    assert_eq!(s.raw("gameplay.traffic").as_deref(), Some("100"));
    assert_eq!(s.raw("gameplay.pedestrians").as_deref(), Some("5"));
    s.restore_session();
    assert_eq!(s.raw("gameplay.pedestrians"), None);
}
