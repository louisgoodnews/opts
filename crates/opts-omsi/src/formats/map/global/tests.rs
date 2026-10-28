//! Unit tests for `super` (`global.rs`).

use super::*;

#[test]
fn test_parse_minimal_global_cfg() {
    let text = r#"
[name]
Grundorf

[friendlyname]
Grundorf

[description]
Grundorf is a small test map.
Enjoy driving!
[end]

[version]
14

[NextIDCode]
322886

[moneysystem]
Money\DM\DM.cfg

[ticketpack]
Ticketpacks\Berlin_1\Berlin_1.otp

[standarddepot]
Grundorf

[map]
0
0
tile_0_0.map

[map]
0
-1
tile_0_-1.map
"#;
    let manifest = MapManifest::from_text(text).unwrap();
    assert_eq!(manifest.name, "Grundorf");
    assert_eq!(manifest.friendly_name.as_deref(), Some("Grundorf"));
    assert_eq!(
        manifest.description,
        vec!["Grundorf is a small test map.", "Enjoy driving!"]
    );
    assert_eq!(manifest.version, Some(14));
    assert_eq!(manifest.next_id_code, Some(322886));
    assert_eq!(manifest.money_system.as_deref(), Some(r"Money\DM\DM.cfg"));
    assert_eq!(
        manifest.ticket_pack.as_deref(),
        Some(r"Ticketpacks\Berlin_1\Berlin_1.otp")
    );
    assert_eq!(manifest.standard_depot.as_deref(), Some("Grundorf"));
    assert_eq!(manifest.tiles.len(), 2);
    assert_eq!(manifest.tiles[0].x, 0);
    assert_eq!(manifest.tiles[0].y, 0);
    assert_eq!(manifest.tiles[0].file_name, "tile_0_0.map");
    assert_eq!(manifest.tiles[1].x, 0);
    assert_eq!(manifest.tiles[1].y, -1);
    assert_eq!(manifest.tiles[1].file_name, "tile_0_-1.map");
}

/// MI-0020: an entry point whose name starts with `#` or is empty must not shift the records
/// behind it (a map with such a depot failed to load: `invalid number for entrypoint id_code`).
#[test]
fn test_entrypoint_names_may_start_with_hash_or_be_empty() {
    let rec = |id: u32, name: &str| {
        format!(
            "{id}\n5464087\n0\n47.929\n-0.010\n45.933\n0.000\n0.707\n0.000\n0.707\n464\n{name}\n"
        )
    };
    let text = format!(
        "[entrypoints]\n3\n{}{}{}\n[map]\n0\n0\ntile_0_0.map\n",
        rec(1, "# Betriebshof Rostorf"),
        rec(2, ""),
        rec(3, "Haltestelle")
    );
    let m = MapManifest::from_text(&text).expect("parses");
    let ids: Vec<u32> = m.entry_points.iter().map(|e| e.id).collect();
    assert_eq!(ids, vec![1, 2, 3]);
    assert_eq!(m.entry_points[0].name, "# Betriebshof Rostorf");
    assert_eq!(m.entry_points[1].name, "");
    assert_eq!(m.entry_points[2].name, "Haltestelle");
    assert_eq!(m.tiles.len(), 1);
}

#[test]
fn test_parse_entrypoints() {
    let text = r#"
[entrypoints]
1
221
48551
0
60.113
33.200
313.728
0.000
-0.449
0.000
0.894
18
Spandau Westerwaldstr.
"#;
    let manifest = MapManifest::from_text(text).unwrap();
    assert_eq!(manifest.entry_points.len(), 1);
    let ep = &manifest.entry_points[0];
    assert_eq!(ep.id, 221);
    assert_eq!(ep.id_code, 48551);
    assert_eq!(ep.position, Vec3::new(60.113, 33.200, 313.728));
    assert_eq!(ep.direction, Vec3::new(-0.449, 0.000, 0.894));
    assert_eq!(ep.name, "Spandau Westerwaldstr.");
}
