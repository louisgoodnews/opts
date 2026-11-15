//! Descriptor tests: well-formed descriptors, unique keys, search and the generated reference.

use super::*;

#[test]
fn every_descriptor_is_well_formed_and_has_both_languages() {
    let r = registry();
    let loc = localization("en");
    for d in r.all() {
        // the default is valid for its own kind
        if d.is_stored() {
            assert!(
                normalize(d, d.default).is_ok(),
                "default of {} invalid",
                d.key
            );
        }
        let mut keys = vec![d.label_key, d.description_key];
        if let SettingKind::Enum(o) = d.kind {
            keys.extend(o.iter().map(|o| o.label_key));
        }
        for k in keys {
            assert!(loc.has("en", k), "{k} missing in en");
            assert!(loc.has("de", k), "{k} missing in de");
        }
    }
    for c in Category::ALL {
        assert!(loc.has("en", c.label_key()) && loc.has("de", c.label_key()));
    }
}

#[test]
fn keys_are_unique_and_never_another_keys_old_name() {
    let r = registry();
    let all = r.all();
    for d in &all {
        for old in d.migrate_from {
            assert!(r.get(old).is_none(), "{old} is both a key and an old name");
        }
    }
}

#[test]
fn search_finds_by_label_keyword_and_description_in_both_languages() {
    let mut r = registry();
    r.register(FIXTURE);
    let mut loc = localization("en");
    let keys = |q: &str, loc: &Localization| -> Vec<&'static str> {
        search::search(&r, loc, q).iter().map(|d| d.key).collect()
    };
    assert!(keys("vsync", &loc).contains(&"graphics.vsync"));
    assert!(keys("tearing", &loc).contains(&"graphics.vsync"), "keyword");
    assert!(
        keys("kilometres", &loc).contains(&"interface.units"),
        "description"
    );
    assert!(keys("steering", &loc).contains(&"fixture.wheel_sensitivity"));
    assert!(
        keys("window size", &loc).contains(&"graphics.resolution"),
        "all words match"
    );
    assert!(keys("zzzz", &loc).is_empty());
    assert!(keys("  ", &loc).is_empty());
    loc.set_locale("de");
    assert!(keys("fenstergröße", &loc).contains(&"graphics.resolution"));
}

#[test]
fn reference_page_is_up_to_date() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/src/reference/settings.md");
    let want = reference::markdown(&registry());
    if std::env::var_os("UPDATE_SETTINGS_DOC").is_some() {
        std::fs::write(&path, &want).expect("write reference page");
    }
    // A Windows checkout (`core.autocrlf`) has CRLF line ends; the page is compared by content.
    let have = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read the reference page: {e}"))
        .replace("\r\n", "\n");
    assert_eq!(
        have, want,
        "docs/src/reference/settings.md is stale: run `UPDATE_SETTINGS_DOC=1 cargo test -p opts-game settings_registry`"
    );
}
