//! Scanning a synthetic map folder and the tile-name keys.

use super::super::*;

/// A synthetic map: two entries (one nested in a group folder), a new tile, a carve marker and
/// data files.
#[test]
fn scan_plan_data_files_and_switch() {
    let root = std::env::temp_dir().join(format!("opts-chrono-scan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let w = |rel: &str, text: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    };
    w("parklist_p.txt", "base");
    w("TTData/x.ttl", "base x");
    w("TTData/y.ttl", "base y");
    w(
        "Chrono/0100_A/Chrono.cfg",
        "[startdate]\r\n19800101\r\n[ticketpack]\r\na.otp\r\n",
    );
    w("Chrono/0100_A/tile_1_1.map", "");
    w("Chrono/0100_A/tile_5_5.map", "");
    w("Chrono/0100_A/tile_1_1.map.chronoterrrelevant", "Dummy");
    w("Chrono/0100_A/PARKLIST_P.txt", "a");
    w(
        "Chrono/grp/0200_B/Chrono.cfg",
        "[startdate]\r\n19900101\r\n[enddate]\r\n19951231\r\n",
    );
    w("Chrono/grp/0200_B/tile_1_1.map", "");
    w("Chrono/grp/0200_B/unsched_trafficdens.txt", "b");
    w("Chrono/grp/0200_B/TTData/X.ttl", "b x");
    w("Chrono/no_cfg/tile_9_9.map", "");
    let d = ChronoDate::from_ymd;
    let set = ChronoSet::scan(&root);
    let rels: Vec<&str> = set.entries().iter().map(|e| e.rel.as_str()).collect();
    assert_eq!(
        rels,
        vec!["0100_A", "grp/0200_B"],
        "a folder without Chrono.cfg is no entry"
    );
    let (early, a, ab) = (
        set.active(d(1975, 1, 1)),
        set.active(d(1985, 1, 1)),
        set.active(d(1992, 1, 1)),
    );
    assert!(early.is_empty());
    assert_eq!(ab.indices(), &[0, 1]);
    let plan = set.plan(&ab);
    assert_eq!(plan.overlays((1, 1)).len(), 2);
    assert!(plan.overlays((1, 1))[1].starts_with(root.join("Chrono/grp")));
    assert_eq!(plan.overlays((5, 5)).len(), 1);
    assert!(plan.terrain_relevant.contains(&(1, 1)));
    assert_eq!(
        set.effective_file(&early, "parklist_p.txt"),
        Some(root.join("parklist_p.txt"))
    );
    assert_eq!(
        set.effective_file(&a, "Parklist_P.txt"),
        Some(root.join("Chrono/0100_A/PARKLIST_P.txt"))
    );
    assert_eq!(set.effective_file(&a, "unsched_trafficdens.txt"), None);
    let tt = set.ttdata(&ab);
    assert_eq!(std::fs::read_to_string(&tt["x.ttl"]).unwrap(), "b x");
    assert_eq!(std::fs::read_to_string(&tt["y.ttl"]).unwrap(), "base y");
    assert_eq!(set.summary(&a).ticketpacks, vec!["a.otp"]);

    assert!(set.switch(&ab, &set.active(d(1993, 6, 1))).is_noop());
    let sw = set.switch(&a, &ab);
    assert_eq!(sw.toggled, vec![1]);
    assert_eq!(sw.tiles.iter().copied().collect::<Vec<_>>(), vec![(1, 1)]);
    assert!(
        sw.carve_ring.is_empty(),
        "the marker belongs to the untoggled entry"
    );
    assert!(sw.touches("UNSCHED_TRAFFICDENS.TXT") && sw.ttdata);
    let sw = set.switch(&early, &a);
    assert_eq!(sw.tiles.len(), 2);
    assert_eq!(sw.carve_ring.len(), 9);
    assert!(sw.carve_ring.contains(&(0, 0)) && sw.carve_ring.contains(&(2, 2)));
    assert!(sw.touches_prefix("parklist_p"));
    assert_eq!(sw.rebuild_tiles().len(), 10);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn tile_names_give_keys() {
    assert_eq!(tile_key_of("tile_2391_11276.map"), Some((2391, 11276)));
    assert_eq!(tile_key_of("Tile_-3_-8.MAP"), Some((-3, -8)));
    assert_eq!(tile_key_of("tile_1_2.map.terrain"), None);
}
