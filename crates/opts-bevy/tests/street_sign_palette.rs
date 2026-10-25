//! `StreetSign_1*.sco` (`Verkehrszeichen_MC`) puts `[matl] StrSchild_Text1.bmp 0` on a two-frame
//! `.x` mesh (plate `StrSchild_1.bmp`, text quad `StrSchild_Text1.bmp`). With the file-wide
//! material palette the override reaches the text quad only; the plate keeps its texture.
//! Skipped without `OMSI_PATH`.

use std::path::PathBuf;

use opts_bevy::model::find_override;
use opts_bevy::{parse_mesh_file, OmsiInstall, TextureOverride};
use opts_core::formats::scenery::SceneryObject;

#[test]
fn street_sign_plate_keeps_its_texture_and_the_text_quad_gets_the_override() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let install = OmsiInstall::new(root);
    let dir = PathBuf::from("Sceneryobjects/Verkehrszeichen_MC");
    let mut checked = 0;
    for name in ["StreetSign_1.sco", "StreetSign_1lg.sco"] {
        let Some(sco_path) = install.find_file(&dir.join(name).to_string_lossy()) else {
            continue;
        };
        let sco = SceneryObject::parse(&std::fs::read(&sco_path).expect("read sco")).expect("sco");
        let model_dir = sco_path.parent().expect("dir").join("model");
        for mesh in &sco.meshes {
            let Some(x) = install.find_in(&model_dir, &mesh.file) else {
                continue;
            };
            let models = parse_mesh_file(&x, &std::fs::read(&x).expect("read x")).expect("x");
            let overrides: Vec<TextureOverride> = mesh
                .materials
                .iter()
                .map(|m| TextureOverride {
                    slot: m.material_index,
                    texture: m.texture.clone(),
                    alpha: m.alpha_mode,
                    alpha_variable: None,
                    trans_map: None,
                    image: None,
                    light_map: None,
                    free_texture: None,
                    night_map: None,
                })
                .collect();
            let palette = &models[0].materials;
            for (slot, m) in palette.iter().enumerate() {
                let native = m.primary_texture().unwrap_or("").to_ascii_lowercase();
                let hit = find_override(palette, slot, &overrides);
                if native.starts_with("strschild_text") {
                    assert!(hit.is_some(), "{name}: text slot {slot} gets the override");
                } else {
                    assert!(
                        hit.is_none(),
                        "{name}: slot {slot} ({native}) keeps its texture"
                    );
                }
                checked += 1;
            }
        }
    }
    eprintln!("street sign slots checked: {checked}");
}
