//! Unit tests of `sco.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

#[test]
fn test_parse_minimal_sco() {
    let text = r#"
[groups]
2
Aachen
Ampeln

[friendlyname]
Ampeln_Fussgaenger

[trafficlight]

[mesh]
Fussgaenger.o3d

[shadow]

[mesh]
Fuss_Rot.o3d

[visible]
red
1

[boundingbox]
0.3
0.3
1.2
0.0
0.0
0.6
"#;
    let sco = SceneryObject::from_text(text).unwrap();
    assert_eq!(sco.groups, vec!["Aachen", "Ampeln"]);
    assert_eq!(sco.friendly_name.as_deref(), Some("Ampeln_Fussgaenger"));
    assert!(sco.is_traffic_light);
    assert_eq!(sco.meshes.len(), 2);
    assert_eq!(sco.meshes[0].file, "Fussgaenger.o3d");
    assert_eq!(sco.meshes[0].cast_shadow, Some(true));
    assert_eq!(sco.meshes[1].file, "Fuss_Rot.o3d");
    assert_eq!(sco.meshes[1].visible, Some(("red".to_string(), 1.0)));

    let bbox = sco.bounding_box.unwrap();
    assert_eq!(bbox.half_extents, Vec3::new(0.3, 0.3, 1.2));
    assert_eq!(bbox.center, Vec3::new(0.0, 0.0, 0.6));
}

#[test]
fn attachment_points_are_numbered_in_file_order() {
    let text = "[new_attachment]\nattach_trans\n0\n0.053\n2.376\n\n[mesh]\nmast.o3d\n\n[new_attachment]\nattach_rot_z\n180\n\nattach_trans\n-0.147\n0.000\n1.120\n";
    let sco = SceneryObject::from_text(text).unwrap();
    assert_eq!(sco.attachments.len(), 2);
    assert_eq!(sco.attachments[0].translation, Vec3::new(0.0, 0.053, 2.376));
    assert_eq!(sco.attachments[0].rotation_deg, Vec3::ZERO);
    assert_eq!(sco.attachments[1].translation, Vec3::new(-0.147, 0.0, 1.12));
    assert_eq!(sco.attachments[1].rotation_deg.z, 180.0);
    assert_eq!(sco.meshes.len(), 1);
}

#[test]
fn test_parse_traffic_light_and_paths() {
    let text = r#"
[traffic_lights_group]
99.99

[traffic_light]
Main
[phase]
6
43.0
[phase]
10
3.0
[phase]
0
52.0

[use_traffic_light]
0

[approachdist]
25.0

[path]
10.0
20.0
0.5
90.0
-999.0
35.0
0.0
0.0
0
3.5
0
"#;
    let sco = SceneryObject::from_text(text).unwrap();
    assert_eq!(sco.traffic_light_groups.len(), 1);
    let grp = &sco.traffic_light_groups[0];
    assert_eq!(grp.cycle_time_sec, 99.99);
    assert_eq!(grp.programs.len(), 1);
    assert_eq!(grp.programs[0].name, "Main");
    assert_eq!(grp.programs[0].phases.len(), 3);
    assert_eq!(grp.programs[0].phases[0].state, 6);
    assert_eq!(grp.programs[0].phases[0].duration_sec, 43.0);

    assert_eq!(sco.paths.len(), 1);
    let path = &sco.paths[0];
    assert_eq!(path.start, Vec3::new(10.0, 20.0, 0.5));
    assert_eq!(path.heading_deg, 90.0);
    assert_eq!(path.radius_m, -999.0);
    assert_eq!(path.length_m, 35.0);
    assert_eq!(path.path_type, 0);
    assert_eq!(path.width_m, 3.5);
    assert_eq!(path.direction, 0);
    assert_eq!(path.traffic_light_index, Some(0));
    assert_eq!(path.approach_distance_m, Some(25.0));
}

#[test]
fn use_traffic_light_governs_the_next_path_only_and_approachdist_belongs_to_the_program() {
    let text = "[traffic_lights_group]\n80\n[traffic_light]\nA\n[phase]\n6\n40\n[phase]\n0\n40\n[approachdist]\n50\n[traffic_light]\nB\n[phase]\n6\n80\n[approachdist]\n0\n\n[path]\n0\n0\n0\n0\n0\n3\n0\n0\n0\n3.5\n0\n[use_traffic_light]\n1\n[path]\n0\n3\n0\n0\n0\n10\n0\n0\n0\n3.5\n0\n[path]\n0\n13\n0\n0\n0\n10\n0\n0\n0\n3.5\n0\n[use_traffic_light]\n0\n[path]\n0\n23\n0\n0\n0\n10\n0\n0\n0\n3.5\n0\n";
    let sco = SceneryObject::from_text(text).unwrap();
    let lights: Vec<_> = sco.paths.iter().map(|p| p.traffic_light_index).collect();
    assert_eq!(lights, [None, Some(1), None, Some(0)]);
    // Approach distances come from the programs (declaration order across groups).
    assert_eq!(
        sco.traffic_light(0).unwrap().1.approach_distance_m,
        Some(50.0)
    );
    assert_eq!(
        sco.traffic_light(1).unwrap().1.approach_distance_m,
        Some(0.0)
    );
    assert!(sco.traffic_light(2).is_none());
    assert_eq!(sco.paths[1].approach_distance_m, Some(0.0));
    assert_eq!(sco.paths[3].approach_distance_m, Some(50.0));
    assert_eq!(sco.traffic_light(0).unwrap().1.cycle_sec(), 80.0);
}

#[test]
fn parses_tree_helper() {
    let sco = SceneryObject::parse(b"[tree]\nTree_Large_04.tga\n20\n30\n0.7\n1.0\n[onlyeditor]\n")
        .expect("parse");
    let tree = sco.tree.expect("tree");
    assert_eq!(tree.texture, "Tree_Large_04.tga");
    assert_eq!((tree.min_height, tree.max_height), (20.0, 30.0));
    assert_eq!((tree.min_width_ratio, tree.max_width_ratio), (0.7, 1.0));
    assert!(sco.only_editor);
}

#[test]
fn onlyeditor_after_a_mesh_hides_that_mesh_only() {
    // `Trees_MC`: helper mesh, then `[onlyeditor]`, then the billboard.
    let sco = SceneryObject::parse(
        b"[mesh]\ntreehelper.x\n[onlyeditor]\n[tree]\nt.tga\n20\n30\n0.7\n1.0\n",
    )
    .expect("parse");
    assert!(!sco.only_editor);
    assert!(sco.meshes[0].only_editor);
    assert!(!sco.mesh_in_game(0));
    assert!(sco.is_editor_only());
    // Two meshes, the second a helper.
    let sco = SceneryObject::parse(b"[mesh]\na.o3d\n[mesh]\nb.o3d\n[onlyeditor]\n").expect("parse");
    assert!(!sco.only_editor);
    assert!(sco.mesh_in_game(0));
    assert!(!sco.mesh_in_game(1));
    assert!(!sco.is_editor_only());
}

#[test]
fn onlyeditor_before_any_mesh_hides_the_object() {
    // `Generic/car_park.sco`.
    let sco = SceneryObject::parse(
        b"[friendlyname]\nCar park\n[carpark_p]\n[onlyeditor]\n[mesh]\nparkplatz.x\n",
    )
    .expect("parse");
    assert!(sco.carpark_p);
    assert!(sco.only_editor);
    assert!(!sco.meshes[0].only_editor);
    assert!(!sco.mesh_in_game(0));
    assert!(sco.is_editor_only());
}

#[test]
fn busstop_marks_the_object_as_a_bus_stop() {
    let sco = SceneryObject::parse(
        b"[friendlyname]\nBus Stop\n[onlyeditor]\n[mesh]\nbusstop.x\n[busstop]\n",
    )
    .expect("parse");
    assert!(sco.bus_stop);
    assert!(
        !SceneryObject::parse(b"[mesh]\na.o3d\n")
            .expect("parse")
            .bus_stop
    );
}
