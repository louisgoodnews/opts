//! Unit tests of the shipped setvar data file and the vendor data of the vehicles.

use super::*;

#[test]
fn the_shipped_data_file_is_sound() {
    let families = curated_families();
    assert_eq!(families.len(), 12, "twelve documented families");
    let mut ids = BTreeSet::new();
    for f in families {
        assert!(ids.insert(&f.id), "duplicate family id {}", f.id);
        assert!(!f.setvars.is_empty(), "{}: no setvars", f.id);
        let mut names = BTreeSet::new();
        for s in &f.setvars {
            assert_eq!(
                s.name,
                s.name.trim(),
                "{}: name not trimmed: {:?}",
                f.id,
                s.name
            );
            assert!(
                names.insert(s.name.to_lowercase()),
                "{}: duplicate {}",
                f.id,
                s.name
            );
        }
    }
    let total: usize = families.iter().map(|f| f.setvars.len()).sum();
    assert_eq!(total, 510);
    let o530 = family_of_folder("citybus 530 by kajosoft").expect("matched case-insensitively");
    assert_eq!(o530.setvars.len(), 83);
    assert!(family_of_folder("No Such Folder").is_none());
    // Plain ranges dominate; the rest is free text that gives no documented range.
    let plain = families
        .iter()
        .flat_map(|f| &f.setvars)
        .filter(|s| parse_range(&s.values).is_some())
        .count();
    assert!(plain > 400, "only {plain} plain ranges");
}

#[test]
fn the_respawn_marker_comes_from_the_documented_data_and_never_from_a_guess() {
    let doc = |name: &str, respawn| CuratedSetvar {
        name: name.into(),
        values: "0|1".into(),
        meaning: String::new(),
        group: String::new(),
        respawn,
    };
    let family = CuratedFamily {
        id: "f".into(),
        title: "F".into(),
        folders: vec![],
        setvars: vec![
            doc("engine", Some(true)),
            doc("roof", Some(false)),
            doc("seats", None),
        ],
    };
    let observed = [sv("a", "engine", "1"), sv("a", "extra", "1")];
    let decl = declared(&[
        ("engine", "l"),
        ("roof", "l"),
        ("seats", "l"),
        ("extra", "l"),
    ]);
    let c = SetvarCatalogue::build(&observed, &decl, Some(&family));
    let at = |n: &str| c.get(n).expect(n).respawn;
    assert_eq!(at("engine"), Respawn::Required);
    assert_eq!(at("roof"), Respawn::Live);
    assert_eq!(at("seats"), Respawn::Unknown);
    assert_eq!(at("extra"), Respawn::Unknown, "undocumented stays unknown");
}

#[test]
fn the_shipped_vendor_data_marks_the_documented_restart_options() {
    let iveco = curated_families()
        .iter()
        .find(|f| f.id == "iveco-interurbain")
        .expect("family");
    for name in ["sv_display_color", "engine_sv_req", "gearbox_sv_req"] {
        let s = iveco.setvars.iter().find(|s| s.name == name).expect(name);
        assert_eq!(s.respawn, Some(true), "{name}");
    }
    let marked: usize = curated_families()
        .iter()
        .flat_map(|f| &f.setvars)
        .filter(|s| s.respawn.is_some())
        .count();
    assert_eq!(
        marked, 4,
        "three documented restarts and one live-tested option"
    );
}
