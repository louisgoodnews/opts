//! The window glass of the MAN SD83 (`SD83_fenster_ext.o3d`, material `Fenster.tga`) is
//! `[matl_alpha] 2` scaled by `Envir_Brightness`. It must follow the variable at run time; the
//! rain and dirt overlays of the same mesh stay out. Skipped without `OMSI_PATH`.

use std::path::PathBuf;

use opts_bevy::TextureOverride;
use opts_core::formats::vehicles::model::ModelConfig;

fn over(m: &opts_core::formats::vehicles::model::ModelMaterial) -> TextureOverride {
    TextureOverride {
        slot: m.slot_index as usize,
        texture: m.texture.clone(),
        alpha: m.alpha_mode,
        alpha_variable: m.alpha_scale_var.clone(),
        trans_map: m.trans_map.clone().filter(|t| !t.trim().is_empty()),
        image: None,
        light_map: None,
        free_texture: None,
        night_map: None,
    }
}

#[test]
fn sd83_window_glass_follows_envir_brightness_and_weather_overlays_stay_out() {
    let Some(root) = std::env::var_os("OMSI_PATH").map(PathBuf::from) else {
        eprintln!("OMSI_PATH not set; skipping");
        return;
    };
    let Ok(bytes) = std::fs::read(root.join("Vehicles/MAN_SD200/Model/model_SD83.cfg")) else {
        return;
    };
    let model = ModelConfig::parse_bytes(&bytes).expect("model_SD83.cfg");
    let mut glass = 0;
    let mut weather = 0;
    let mut followed = 0;
    let mut dropped = 0;
    for lod in &model.lods {
        for mesh in &lod.meshes {
            for m in &mesh.materials {
                let o = over(m);
                if o.runtime_alpha_variable().is_some() {
                    followed += 1;
                }
                if o.is_variable_hidden() {
                    dropped += 1;
                }
                let is_exterior_window = mesh
                    .path
                    .to_ascii_lowercase()
                    .ends_with("sd83_fenster_ext.o3d");
                if is_exterior_window && m.texture.eq_ignore_ascii_case("Fenster.tga") {
                    assert_eq!(o.runtime_alpha_variable(), Some("Envir_Brightness"));
                    assert!(!o.is_variable_hidden());
                    glass += 1;
                }
                if is_exterior_window
                    && (m.texture.eq_ignore_ascii_case("regen.tga")
                        || m.texture.eq_ignore_ascii_case("SD_dreck.tga"))
                {
                    assert!(o.is_variable_hidden(), "{}", m.texture);
                    weather += 1;
                }
            }
        }
    }
    eprintln!("SD83: {followed} slots follow a variable, {dropped} weather overlays dropped");
    assert!(glass >= 1, "a glass slot on the exterior window mesh");
    assert!(weather >= 4, "rain and dirt overlays: {weather}");
    assert!(followed > 0 && dropped > 0);
}
