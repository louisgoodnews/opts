//! Profile files, validation, presets, and the fallback chain.

use super::super::profile::{chrono_id, slug, Dropped};

use super::*;

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
