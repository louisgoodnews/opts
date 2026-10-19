//! Unit tests of `bus.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;

#[test]
fn coupling_front_character_is_parsed() {
    let m = BusManifest::parse(
        "[coupling_front]\n0\n4.5\n0.3\n\n[coupling_front_character]\n52.5\n-20\n20\n1\n",
    )
    .unwrap();
    let l = m.coupling_front_limits.expect("limits");
    assert_eq!(
        (l.max_alpha_deg, l.min_beta_deg, l.max_beta_deg, l.kind),
        (52.5, -20.0, 20.0, 1)
    );
    assert_eq!(m.coupling_front.map(|p| p.y), Some(4.5));
}

#[test]
fn test_parse_sample_bus() {
    let sample = r#"
[friendlyname]
MAN
NL202
BVG Standard

[description]
Standard Berliner Linienbus
Baujahr 1992
[end]

[number]
Regs_NL202.org

[sound]
sound\sound_NL202.cfg

[paths]
model\paths_NL202.cfg

[passengercabin]
model\passengercabin_NL202.cfg

[model]
model\model_NL202.cfg

[varnamelist]
2
script\engine_varlist.txt
script\door_varlist.txt

[script]
1
script\engine.osc

[constfile]
1
script\engine_constfile.txt

[mass]
11.5

[newachse]
achse_long
3.238
achse_maxwidth
2.25
achse_antrieb
1
"#;
    let bus = parse_bus(sample).unwrap();
    assert_eq!(
        bus.friendly_name,
        Some(FriendlyName {
            manufacturer: "MAN".to_string(),
            model: "NL202".to_string(),
            livery: "BVG Standard".to_string(),
        })
    );
    assert_eq!(
        bus.description,
        vec!["Standard Berliner Linienbus", "Baujahr 1992"]
    );
    assert_eq!(bus.number_file, Some("Regs_NL202.org".to_string()));
    assert_eq!(bus.sound_cfg, Some(r"sound\sound_NL202.cfg".to_string()));
    assert_eq!(bus.paths_cfg, Some(r"model\paths_NL202.cfg".to_string()));
    assert_eq!(bus.varnamelists.len(), 2);
    assert_eq!(bus.scripts.len(), 1);
    assert_eq!(bus.constfiles.len(), 1);
    assert_eq!(bus.mass_tonnes, Some(11.5));
    assert_eq!(bus.axles.len(), 1);
    assert_eq!(bus.axles[0].longitudinal_m, 3.238);
    assert_eq!(bus.axles[0].max_width_m, Some(2.25));
    assert!(bus.axles[0].is_driven);
}

#[test]
fn test_parse_bus_cameras() {
    let sample = r#"
[add_camera_driver]
-0.70
4.40
1.80
-0.06
55.0
0.0
-5.0

[view_schedule]

[add_camera_driver]
-0.70
4.40
1.80
-0.06
55.0
-45.0
-15.0

[view_ticketselling]

[set_camera_std]
0

[add_camera_pax]
0.0
2.0
1.7
0.0
65.0
0.0
0.0

[set_camera_outside_center]
0.0
0.5
1.5
"#;
    let bus = parse_bus(sample).unwrap();
    assert_eq!(bus.driver_cameras.len(), 2);
    assert_eq!(bus.driver_cameras[0].x, -0.70);
    assert_eq!(bus.driver_cameras[0].y, 4.40);
    assert_eq!(bus.driver_cameras[0].z, 1.80);
    assert_eq!(bus.driver_cameras[0].dist, -0.06);
    assert_eq!(bus.driver_cameras[0].fov_deg, 55.0);
    assert_eq!(bus.driver_cameras[0].yaw_deg, 0.0);
    assert_eq!(bus.driver_cameras[0].pitch_deg, -5.0);
    assert!(bus.driver_cameras[0].is_driver);
    assert!(bus.driver_cameras[0].is_schedule_view);
    assert!(!bus.driver_cameras[0].is_ticket_view);

    assert_eq!(bus.driver_cameras[1].yaw_deg, -45.0);
    assert!(bus.driver_cameras[1].is_ticket_view);

    assert_eq!(bus.std_camera_index, Some(0));

    assert_eq!(bus.pax_cameras.len(), 1);
    assert_eq!(bus.pax_cameras[0].y, 2.0);
    assert!(!bus.pax_cameras[0].is_driver);

    assert_eq!(
        bus.outside_camera_center,
        Some(glam::Vec3::new(0.0, 0.5, 1.5))
    );
}

#[test]
fn mirror_cameras_keep_file_order_across_both_keywords() {
    let bus = parse_bus(
        "[add_camera_reflexion_2]\n-1.3\n5.4\n2.1\n0\n52\n169\n-10\n0.15\n\n\
         [add_camera_reflexion]\n1.32\n5.78\n2.2\n0\n52\n201\n-5\n\n\
         [add_camera_reflexion]\n1\n2\n3\n0\nx\n195\n-13\n",
    )
    .unwrap();
    // The third has an unreadable field: dropped, not an error.
    assert_eq!(bus.mirror_cameras.len(), 2);
    let (a, b) = (&bus.mirror_cameras[0], &bus.mirror_cameras[1]);
    assert_eq!((a.x, a.y, a.z), (-1.3, 5.4, 2.1));
    assert_eq!((a.fov_deg, a.yaw_deg, a.pitch_deg), (52.0, 169.0, -10.0));
    assert_eq!(a.param8, Some(0.15));
    assert_eq!(b.param8, None);
    assert_eq!(b.yaw_deg, 201.0);
}

#[test]
fn mirror_cameras_of_installed_vehicles() {
    let Some(root) = std::env::var_os("OMSI_PATH") else {
        return;
    };
    let dir = std::path::Path::new(&root).join("Vehicles");
    let read = |rel: &str| std::fs::read(dir.join(rel)).ok();
    if let Some(bytes) = read("MAN_SD202/MAN_D86.bus") {
        let bus = BusManifest::parse_bytes(&bytes).unwrap();
        assert_eq!(bus.mirror_cameras.len(), 3);
        let m = &bus.mirror_cameras[0];
        assert_eq!((m.x, m.y, m.z), (-1.342, 5.345, 1.799));
        assert_eq!((m.fov_deg, m.yaw_deg, m.pitch_deg), (52.0, 169.0, 0.0));
        assert_eq!(m.param8, Some(0.15));
        assert_eq!(bus.mirror_cameras[1].param8, None);
        assert_eq!(bus.mirror_cameras[1].yaw_deg, 201.0);
    }
    if let Some(bytes) = read("Gladbeck_Stadtbus/GladbeckStadtbus2015.bus") {
        let bus = BusManifest::parse_bytes(&bytes).unwrap();
        assert_eq!(bus.mirror_cameras.len(), 6, "two door cameras included");
        assert_eq!(bus.mirror_cameras[0].x, -1.8, "left outside mirror");
        assert_eq!(bus.mirror_cameras[4].param8, Some(6.0));
    }
}
