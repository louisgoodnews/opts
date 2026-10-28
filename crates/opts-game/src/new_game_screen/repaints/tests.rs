//! Unit tests for `super` (`repaints.rs`).

use super::*;

fn tmp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("opts_repaints_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Whether `dir` is on a case-sensitive file system (on Windows `texture` and `Texture` are one
/// folder).
fn case_sensitive(dir: &std::path::Path) -> bool {
    let probe = dir.join("case_probe");
    std::fs::write(&probe, b"x").unwrap();
    let sensitive = !dir.join("CASE_PROBE").exists();
    let _ = std::fs::remove_file(probe);
    sensitive
}

#[test]
fn folders_that_differ_in_case_are_all_found_and_their_repaints_merge() {
    let root = tmp_root("case");
    let veh = root.join("Vehicles/V");
    std::fs::create_dir_all(veh.join("texture/Ads")).unwrap();
    std::fs::create_dir_all(veh.join("Texture/ads")).unwrap();
    std::fs::write(
        veh.join("texture/Ads/One.cti"),
        "[item]\nOne\nfarbschema_tex1\none.dds\n",
    )
    .unwrap();
    std::fs::write(
        veh.join("Texture/ads/Two.cti"),
        "[item]\nTwo\nfarbschema_tex1\ntwo.dds\n",
    )
    .unwrap();
    std::fs::write(veh.join("Texture/ads/two.dds"), b"x").unwrap();
    let dirs = dirs_ignoring_case(&veh, "Texture\\Ads");
    assert_eq!(
        dirs.len(),
        if case_sensitive(&root) { 2 } else { 1 },
        "{dirs:?}"
    );
    assert!(dirs_ignoring_case(&veh, "Texture\\Missing").is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_vehicles_texture_folders_include_every_case_variant() {
    let root = tmp_root("tex");
    let veh = root.join("V");
    std::fs::create_dir_all(veh.join("texture")).unwrap();
    std::fs::create_dir_all(veh.join("Texture")).unwrap();
    std::fs::create_dir_all(veh.join("Model/texture")).unwrap();
    let dirs = texture_dirs(&veh, &veh.join("Model"));
    assert_eq!(
        dirs.len(),
        if case_sensitive(&root) { 3 } else { 2 },
        "{dirs:?}"
    );
    let none = texture_dirs(&root.join("W"), &root.join("W/Model"));
    assert_eq!(none, [root.join("W/texture")]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn default_first_then_cti_items_of_the_colorscheme_folder() {
    let root = tmp_root("a");
    let veh = root.join("Vehicles/V");
    std::fs::create_dir_all(veh.join("Model")).unwrap();
    std::fs::create_dir_all(veh.join("Texture/Ads")).unwrap();
    std::fs::write(
        veh.join("v.bus"),
        "[friendlyname]\nMAN\nX\nBeige\n[model]\nModel\\m.cfg\n",
    )
    .unwrap();
    std::fs::write(
        veh.join("Model/m.cfg"),
        "[CTC]\nColorscheme\nTexture\\Ads\n0\n[CTCTexture]\ntex1\na.tga\n[CTC]\nOther\nTexture\n0\n",
    )
    .unwrap();
    std::fs::write(
        veh.join("Texture/Ads/b.CTI"),
        "[item]\n Zebra\ntex1\nz.tga\n[setvar]\nlights\n1\n[item]\nBeige\ntex1\nb.tga\n[item]\nAlpha\ntex1\nal.tga\n",
    )
    .unwrap();
    let r = discover(&root, "Vehicles/V/v.bus", "Beige");
    assert_eq!(r.names, ["Beige", "Alpha", "Zebra"]);
    assert_eq!(
        r.setvars["Zebra"],
        [("lights".to_string(), "1".to_string())]
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn instance_choice_is_deterministic_and_covers_all_repaints() {
    let r = VehicleRepaints {
        default_name: String::new(),
        names: vec!["Rot".into(), "Gold".into(), "Taxi".into()],
        ..Default::default()
    };
    let a: Vec<&str> = (0..64).map(|i| r.pick(instance_seed(&[7, i]))).collect();
    let b: Vec<&str> = (0..64).map(|i| r.pick(instance_seed(&[7, i]))).collect();
    assert_eq!(a, b);
    for name in ["", "Rot", "Gold", "Taxi"] {
        assert!(a.contains(&name), "{name} chosen at least once");
    }
    assert_ne!(instance_seed(&[1, 2]), instance_seed(&[2, 1]));
    assert_eq!(name_seed("W123"), name_seed("W123"));
    // A named default is one of the choices, not an extra unpainted one.
    let named = VehicleRepaints {
        default_name: "Beige".into(),
        names: vec!["Beige".into()],
        ..Default::default()
    };
    assert_eq!(named.pick(12345), "Beige");
    assert_eq!(VehicleRepaints::default().pick(3), "");
}

#[test]
fn model_repaints_resolve_the_colorscheme_folder_from_the_sco_folder() {
    let root = tmp_root("c");
    let veh = root.join("Vehicles/Car");
    std::fs::create_dir_all(veh.join("Model")).unwrap();
    std::fs::create_dir_all(veh.join("Texture")).unwrap();
    std::fs::write(veh.join("Texture/red.tga"), b"x").unwrap();
    std::fs::write(
        veh.join("Texture/l.cti"),
        "[item]\nRot\nfarbschema\nred.tga\n",
    )
    .unwrap();
    let model = ModelConfig::parse_bytes(
        b"[CTC]\nColorscheme\nTexture\n0\n[CTCTexture]\nfarbschema\nCar_Main.tga\n",
    )
    .unwrap();
    let r = discover_for_model(&root, &veh, &model, "");
    assert_eq!(r.names, ["Rot"]);
    let repl = r.replacements("Rot", &model.ctc_textures);
    assert_eq!(repl.len(), 1);
    assert_eq!(repl[0].0, "car_main.tga");
    assert!(r.replacements("", &model.ctc_textures).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn missing_bus_lists_only_the_default() {
    let root = tmp_root("b");
    let r = discover(&root, "Vehicles/none.bus", "Beige");
    assert_eq!(r.names, ["Beige"]);
    let _ = std::fs::remove_dir_all(&root);
}

/// Needs an installation: `OMSI_PATH=<root> cargo test -p opts-game repaints -- --ignored`.
#[test]
#[ignore]
fn real_installation_lists_gn92_repaints() {
    let root = PathBuf::from(std::env::var("OMSI_PATH").expect("OMSI_PATH"));
    let r = discover(&root, "Vehicles/MAN_NL_NG/MAN_GN92_main.bus", "Beige");
    eprintln!("{} repaints: {:?}", r.names.len(), r.names);
    assert_eq!(r.names[0], "Beige");
    assert!(r.names.len() > 5);
}
