//! Unit tests of `entry.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

#[test]
fn dates_parse_order_and_convert() {
    assert_eq!(
        ChronoDate::parse("19891110"),
        Some(ChronoDate::from_ymd(1989, 11, 10))
    );
    assert_eq!(ChronoDate::parse(""), None);
    assert_eq!(ChronoDate::parse("1989-11-10"), None);
    assert!(ChronoDate::from_ymd(1989, 11, 10) < ChronoDate::from_ymd(1990, 1, 1));
    // 2000-01-01 is day 10957 after 1970-01-01.
    assert_eq!(
        ChronoDate::from_unix_days(10_957),
        ChronoDate::from_ymd(2000, 1, 1)
    );
    assert_eq!(
        ChronoDate::from_unix_days(10_957 - 4629),
        ChronoDate::from_ymd(1987, 4, 30)
    );
    assert_eq!(ChronoDate::from_unix_days(0).to_string(), "1970-01-01");
    assert_eq!(ChronoDate::from_unix_days(-1).to_string(), "1969-12-31");
    assert_eq!(ChronoDate::from_unix_days(11_016).to_string(), "2000-02-29");
}

/// The grammar of `Chrono.cfg`: dates, name, description up to `[end]` (with blank lines),
/// deactivated lines, repeated ticket packs, money system.
#[test]
fn chrono_cfg_fields() {
    let text = "\r\n\r\n[name]\r\nFahrplanwechsel 23.5.1993\r\n\r\n[description]\r\n* Neue KI-Linie\r\n\r\nzweite Zeile\r\n[end]\r\n\r\n[startdate]\r\n19930523\r\n\r\n[enddate]\r\n19940528\r\n[deactivate_lines]\r\n2\r\nRelais\r\nE\r\n[ticketpack]\r\nTicketpacks\\a.otp\r\n[ticketpack]\r\nTicketpacks\\b.otp\r\n\r\n[moneysystem]\r\nMoney\\DM\\DM_93.cfg";
    let mut e = ChronoEntry::default();
    e.read_cfg(text.as_bytes());
    assert_eq!(e.name.as_deref(), Some("Fahrplanwechsel 23.5.1993"));
    assert_eq!(
        e.description.as_deref(),
        Some("* Neue KI-Linie\n\nzweite Zeile")
    );
    assert_eq!(e.start, Some(ChronoDate::from_ymd(1993, 5, 23)));
    assert_eq!(e.end, Some(ChronoDate::from_ymd(1994, 5, 28)));
    assert_eq!(e.deactivate_lines, vec!["Relais", "E"]);
    assert_eq!(
        e.ticketpacks,
        vec!["Ticketpacks\\a.otp", "Ticketpacks\\b.otp"]
    );
    assert_eq!(e.moneysystem.as_deref(), Some("Money\\DM\\DM_93.cfg"));
    // cp1252 is decoded.
    let mut e = ChronoEntry::default();
    e.read_cfg(b"[name]\r\nGr\xfcn\r\n");
    assert_eq!(e.name.as_deref(), Some("Grün"));
}

/// Active ⇔ start ≤ date ≤ end, with the end inclusive and missing bounds open.
#[test]
fn activation_is_inclusive_and_open_ended() {
    let d = ChronoDate::from_ymd;
    let e = ChronoEntry {
        start: Some(d(1989, 11, 10)),
        end: Some(d(1990, 6, 30)),
        ..ChronoEntry::default()
    };
    assert!(!e.is_active(d(1989, 11, 9)));
    assert!(e.is_active(d(1989, 11, 10)));
    assert!(e.is_active(d(1990, 6, 30)));
    assert!(!e.is_active(d(1990, 7, 1)));
    let open = ChronoEntry::default();
    assert!(open.is_active(d(1, 1, 1)) && open.is_active(d(9999, 12, 31)));
}

/// Order: start date (missing first), then folder path case-insensitively.
#[test]
fn entries_are_applied_by_start_date_then_folder() {
    let d = ChronoDate::from_ymd;
    let mk = |rel: &str, start: Option<ChronoDate>| ChronoEntry {
        rel: rel.into(),
        start,
        ..ChronoEntry::default()
    };
    let set = ChronoSet::from_entries(
        Path::new("/x"),
        vec![
            mk("b", Some(d(1990, 1, 1))),
            mk("Z_undated", None),
            mk("A", Some(d(1990, 1, 1))),
            mk("c", Some(d(1980, 1, 1))),
        ],
    );
    let order: Vec<&str> = set.entries().iter().map(|e| e.rel.as_str()).collect();
    assert_eq!(order, vec!["Z_undated", "c", "A", "b"]);
    assert_eq!(set.active(d(1985, 1, 1)).indices(), &[0, 1]);
}

#[test]
fn display_names_prefer_the_language() {
    let e = ChronoEntry {
        rel: "grp/0200_Grenzoeffnung".into(),
        localized_names: vec![
            ("DEU".into(), "Grenzöffnung".into()),
            ("ENG".into(), "Fall of the Wall".into()),
        ],
        ..ChronoEntry::default()
    };
    assert_eq!(e.display_name("DEU"), "Grenzöffnung");
    assert_eq!(e.display_name("FRA"), "Fall of the Wall");
    let bare = ChronoEntry {
        rel: "grp/0200_Grenzoeffnung".into(),
        ..ChronoEntry::default()
    };
    assert_eq!(bare.display_name("ENG"), "0200_Grenzoeffnung");
}

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
