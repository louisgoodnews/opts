//! Unit tests of `tile.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

/// Two line modules on a bus-stop sign (`Aachen Linie 33`, tile 1094/10762), one in version 10.
#[test]
fn attached_objects_name_their_parent_and_attachment_point() {
    let v14 = "[version]\n14\n\n[attachObj]\n0\nSceneryobjects\\ASEAG\\Haltestelle_Linienmodul.sco\n15988\n15986\n0\n1\n90\n0\n0\n2\n396\nVaals Busstation\n\nObject Nr. 29\n";
    let tile = MapTile::from_text(v14).unwrap();
    let a = &tile.attached_objects[0];
    assert_eq!((a.id_code, a.parent_id, a.attachment), (15988, 15986, 1));
    assert_eq!(a.heading_deg, 90.0);
    assert_eq!((a.pitch_deg, a.roll_deg), (Some(0.0), Some(0.0)));
    assert_eq!(a.string_vars, vec!["396", "Vaals Busstation"]);

    let v10 = "[version]\n10\n\n[attachObj]\n0\nSceneryobjects\\X\\Std_H_Name.sco\n42468\n42465\n0\n3\n180\n1\nSüdermarkt\n\nObject Nr. 264\n";
    let tile = MapTile::from_text(v10).unwrap();
    let a = &tile.attached_objects[0];
    assert_eq!(
        (a.parent_id, a.attachment, a.heading_deg),
        (42465, 3, 180.0)
    );
    assert_eq!((a.pitch_deg, a.roll_deg), (None, None));
    assert_eq!(a.string_vars, vec!["Südermarkt"]);
}

/// A blank string value (the `car_park` parking-list selector, Berlin-Spandau) stays blank: the
/// `Object Nr.` comment after it is not pulled into its place.
#[test]
fn blank_string_value_is_kept() {
    let text = "[version]\n14\n\n[object]\n0\nSceneryobjects\\Generic\\car_park.sco\n42\n1\n2\n0\n90\n0\n0\n1\n\n\nObject Nr. 7\n\n[object]\n0\nSceneryobjects\\Generic\\car_park.sco\n43\n1\n2\n0\n90\n0\n0\n6\n\n1\n1\n\n\n\n\nObject Nr. 8\n";
    let tile = MapTile::from_text(text).unwrap();
    assert_eq!(tile.objects.len(), 2);
    assert_eq!(tile.objects[0].string_vars, vec![String::new()]);
    assert_eq!(tile.objects[1].string_vars, vec!["", "1", "1", "", "", ""]);
}

/// `[splineAttachement]` and `[splineAttachement_repeater]` (version 14, Berlin-Spandau tile
/// 2404/11277 style): spacing, range, the repeater's master tile and first instance, strings.
#[test]
fn attachment_layout_with_repeater() {
    let text = "[version]\n14\n\n\
[splineAttachement]\n0\nSceneryobjects\\Generic\\car_park.sco\n1001\n2\n4.25\n0.1\n12.5\n90\n0\n-3\n7\n140\n1\n1\n\n\nObject Nr. 5\n\n\
[splineAttachement_repeater]\n0\n17\n5\nSceneryobjects\\Generic\\car_park.sco\n1001\n0\n4.25\n0.1\n12.5\n90\n0\n-3\n7\n140\n1\n1\n1\nObject Nr. 6\n";
    let tile = MapTile::from_text(text).unwrap();
    let [master, rep] = tile.spline_attachments.as_slice() else {
        panic!("two attachments: {:?}", tile.spline_attachments);
    };
    assert_eq!(
        (master.spline_index, master.lateral_m, master.distance_m),
        (2, 4.25, 12.5)
    );
    assert_eq!(
        (
            master.field8,
            master.field9,
            master.spacing_m,
            master.range_m,
            master.field12
        ),
        (0.0, -3.0, 7.0, 140.0, 1.0)
    );
    assert_eq!(master.repeater, None);
    assert_eq!(master.string_vars, vec![String::new()]);
    assert_eq!(master.instances(), 0..=20);
    assert!((master.chainage(3) - 33.5).abs() < 1e-9);
    assert_eq!(
        rep.repeater,
        Some(RepeaterLink {
            master_tile: 17,
            first_instance: 5
        })
    );
    assert_eq!(rep.spline_index, 0);
    assert_eq!(rep.instances(), 5..=20);
    assert_eq!(rep.string_vars, vec!["1"]);
}

/// Version 10 attachments have no `field8`, `field9` and `field12` (Neuendorf `tile_0_-3.map`).
#[test]
fn version_10_attachment_layout() {
    let text = "[version]\n10\n\n[splineAttachement]\n0\nSceneryobjects\\Generic\\car_park.sco\n8852\n2\n-4.37\n0.15\n8.45\n180\n5.5\n20\n4\nTree_Large_03.tga\n28.400\n1.073\n\n\nObject Nr. 155\n";
    let tile = MapTile::from_text(text).unwrap();
    let a = &tile.spline_attachments[0];
    assert_eq!((a.spacing_m, a.range_m), (5.5, 20.0));
    assert_eq!(a.instances(), 0..=3);
    assert_eq!(
        a.string_vars,
        vec!["Tree_Large_03.tga", "28.400", "1.073", ""]
    );
}

/// A single attachment: no spacing or range.
#[test]
fn single_attachment_has_one_instance() {
    let text =
        "[version]\n14\n\n[splineAttachement]\n0\na.sco\n1\n0\n0\n0\n10\n0\n0\n0\n0\n0\n0\n0\n";
    let tile = MapTile::from_text(text).unwrap();
    assert_eq!(tile.spline_attachments[0].instances(), 0..=0);
    assert!(!tile.spline_attachments[0].is_run());
}

/// Neuenbreid `tile_9_-19.map` (attachments without a carrier) and `tile_3_11.map` (a record cut
/// off by NUL bytes): left out, but named in `warnings`.
#[test]
fn records_that_cannot_be_placed_are_reported() {
    let text = "[version]\n14\n\n[splineAttachement]\n0\nSceneryobjects\\Trees_MC\\shrubbery_04_05.sco\n855395\n-1\n2.0\n-0.5\n1.7\n90\n0\n0\n\n[object]\n0\nSceneryobjects\\Trees_MC\\fir\n";
    let tile = MapTile::from_text(text).unwrap();
    assert!(tile.spline_attachments.is_empty());
    assert!(tile.objects.is_empty());
    assert_eq!(tile.warnings.len(), 2, "{:?}", tile.warnings);
    assert!(tile.warnings[0].contains("names no spline"));
    assert!(tile.warnings[1].contains("[object]"));
}

#[test]
fn test_parse_minimal_tile() {
    let text = r#"
File created with test

[version]
14

[terrain]

[object]
0
Sceneryobjects\Example\house.sco
100001
150.0
120.0
5.0
90.0
0.0
0.0
0

[spline]
0
Splines\Example\road.sli
200001
0
0
10.0
2.5
20.0
180.0
50.0
0.0
0
0
0
0
0.0
0.0
0
"#;
    let tile = MapTile::from_text(text).unwrap();
    assert_eq!(tile.version, Some(14));
    assert!(tile.has_terrain);

    assert_eq!(tile.objects.len(), 1);
    let obj = &tile.objects[0];
    assert_eq!(obj.sco_file, r"Sceneryobjects\Example\house.sco");
    assert_eq!(obj.id_code, 100001);
    assert_eq!(obj.position, Vec3::new(150.0, 120.0, 5.0));
    assert_eq!(obj.heading_deg, 90.0);
    assert_eq!(obj.pitch_deg, Some(0.0));
    assert_eq!(obj.roll_deg, Some(0.0));

    assert_eq!(tile.splines.len(), 1);
    let spl = &tile.splines[0];
    assert_eq!(spl.sli_file, r"Splines\Example\road.sli");
    assert_eq!(spl.id_code, 200001);
    // Position order: Line 5 (10.0) is X, Line 6 (2.5) is Z (elevation), Line 7 (20.0) is Y!
    assert_eq!(spl.position, Vec3::new(10.0, 20.0, 2.5));
    assert_eq!(spl.heading_deg, 180.0);
    assert_eq!(spl.length_m, 50.0);
    assert_eq!(spl.radius_m, 0.0);
}

/// MI-0021: a version 10 tile has one connection field; `-1` places the spline, `0` continues
/// the previous one (all pose fields are zero).
#[test]
fn test_version_10_splines_are_placed_or_continue_the_previous() {
    let rec = |id: u32, conn: &str, x: &str, y: &str, h: &str, len: &str| {
        format!(
            "[spline]\n0\nSplines\\a.sli\n{id}\n{conn}\n{x}\n0\n{y}\n{h}\n{len}\n0\n0\n0\n0\n0\n\n"
        )
    };
    let text = format!(
        "File created\n\n[version]\n10\n\n{}{}",
        rec(1, "-1", "100", "200", "90", "30"),
        rec(2, "0", "0", "0", "0", "10")
    ) + "[rule]\n0\ntrafficdensity\n3.000\n\n[rule]\n1\nspeedlimit\n30\nObject Nr. 0\n";
    let tile = MapTile::from_text(&text).expect("v10 tile parses");
    assert_eq!(tile.rules.len(), 2, "v10 rules have no flag line");
    assert_eq!(tile.rules[1].flag, 0);
    assert_eq!(tile.splines.len(), 2);
    assert_eq!(tile.splines[0].position.x, 100.0);
    assert_eq!(tile.splines[0].position.y, 200.0);
    assert_eq!(tile.splines[0].length_m, 30.0);
    // heading 90 degrees runs towards +x: the second spline starts 30 m east of the first
    let second = &tile.splines[1];
    assert!(
        (second.position.x - 130.0).abs() < 1e-3,
        "{:?}",
        second.position
    );
    assert!((second.position.y - 200.0).abs() < 1e-3);
    assert!((second.heading_deg - 90.0).abs() < 1e-3);
    assert_eq!(second.length_m, 10.0);
}

#[test]
fn test_spline_connection_minus_one_means_unconnected() {
    let text =
        "[version]\n14\n\n[spline]\n0\nSplines\\a.sli\n77\n-1\n-1\n1.0\n2.0\n3.0\n0.0\n10.0\n0.0\n";
    let tile = MapTile::from_text(text).expect("-1 connections parse");
    assert_eq!(tile.splines.len(), 1);
    assert_eq!(tile.splines[0].conn_id_1, 0);
    assert_eq!(tile.splines[0].conn_id_2, 0);
}

#[test]
fn terrain_align_marks_the_spline_before_it() {
    let spline = |id: u32| {
        format!(
            "[spline]\n0\nSplines\\a.sli\n{id}\n0\n0\n1.0\n-12.0\n3.0\n0.0\n10.0\n0.0\n0\n0\n\n"
        )
    };
    let text = format!(
        "[version]\n14\n\n{}[spline_terrain_align_2]\n1\n\n{}[spline_terrain_align]\n0\n\n{}",
        spline(12663),
        spline(12664),
        spline(12665)
    );
    let tile = MapTile::from_text(&text).expect("aligned splines parse");
    assert_eq!(tile.splines.len(), 3);
    assert_eq!(tile.terrain_aligned, vec![12663], "value 0 is no alignment");
}
