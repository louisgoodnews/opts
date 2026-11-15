//! Unit tests for `super` (`corpus.rs`).

use super::super::{object_key, ObjectEntry};

use super::*;

fn entry(sco: &str, tier: &str) -> ObjectEntry {
    ObjectEntry {
        sco: sco.into(),
        key: object_key(sco),
        tier: tier.into(),
    }
}

#[test]
fn selection_filters_tier_glob_sample_and_limit() {
    let all: Vec<_> = (0..100)
        .map(|i| {
            entry(
                &format!("Sceneryobjects/P{}/o{i}.sco", i % 3),
                if i % 2 == 0 {
                    "referenced"
                } else {
                    "unreferenced"
                },
            )
        })
        .collect();
    let sel = Selection {
        tier: "referenced".into(),
        filter: Some("sceneryobjects/p0/*".into()),
        ..Selection::default()
    };
    let got = select(all.clone(), &sel).unwrap();
    assert!(got
        .iter()
        .all(|e| e.tier == "referenced" && e.sco.contains("/P0/")));
    let sampled = select(
        all,
        &Selection {
            tier: "all".into(),
            sample: Some(10),
            limit: Some(7),
            ..Selection::default()
        },
    )
    .unwrap();
    assert_eq!(sampled.len(), 7);
}

#[test]
fn enumerates_sco_files_and_tiers() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("Sceneryobjects/Pack")).unwrap();
    std::fs::create_dir_all(root.join("maps/Town")).unwrap();
    std::fs::write(root.join("Sceneryobjects/Pack/a.sco"), "[mesh]\na.o3d\n").unwrap();
    std::fs::write(root.join("Sceneryobjects/Pack/b.sco"), "[mesh]\nb.o3d\n").unwrap();
    std::fs::write(
        root.join("maps/Town/tile_0_0.map"),
        "[version]\n14\n[object]\n0\nSceneryobjects\\PACK\\A.sco\n1\n0\n0\n0\n0\n0\n0\n0\n",
    )
    .unwrap();
    let e = entries(root).unwrap();
    assert_eq!(e.len(), 2);
    assert_eq!(e[0].tier, "referenced");
    assert_eq!(e[1].tier, "unreferenced");
}
