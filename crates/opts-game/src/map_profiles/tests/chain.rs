//! Tests of the fallback chain: chrono, map, then global, and broken files.


use super::*;


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
