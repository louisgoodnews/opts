//! Mirror panes on real vehicles: the identification the mirror cameras rely on
//! ([`opts_bevy::glass::classify`], pane texture `reflexion<N>.bmp`) applied to the first LOD of a
//! vehicle, looking at the textures inside the `.o3d` files and at the faces that really use them
//! (the loader makes a part, and so a mirror pane, only for a material with faces). Skipped
//! without `OMSI_PATH`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opts_bevy::glass::{classify, SurfaceKind};
use opts_core::formats::vehicles::model::ModelConfig;

fn root() -> Option<PathBuf> {
    let root = std::env::var_os("OMSI_PATH").map(PathBuf::from);
    if root.is_none() {
        eprintln!("OMSI_PATH not set; skipping");
    }
    root
}

/// Pane textures of the first LOD with the number of mesh files that have faces on them, and
/// the number of meshes of that LOD.
fn panes(vehicle_root: &Path, cfg: &Path) -> Option<(BTreeMap<String, usize>, usize)> {
    let bytes = std::fs::read(cfg).ok()?;
    let model = ModelConfig::parse_bytes(&bytes).ok()?;
    let lod0 = model.lods.first()?;
    let mut found: BTreeMap<String, usize> = BTreeMap::new();
    for mesh in &lod0.meshes {
        let rel = mesh.path.replace('\\', "/");
        let Ok(files) = opts_bevy::load_decoded_meshes(&vehicle_root.join(&rel)) else {
            continue;
        };
        for o in &files {
            let mut used: Vec<String> = Vec::new();
            for slot in o.face_materials.iter().copied() {
                let Some(tex) = o
                    .materials
                    .get(slot as usize)
                    .and_then(|m| m.primary_texture())
                else {
                    continue;
                };
                if classify(tex, false) == Some(SurfaceKind::Mirror) {
                    let tex = tex.to_lowercase();
                    if !used.contains(&tex) {
                        used.push(tex);
                    }
                }
            }
            for t in used {
                *found.entry(t).or_default() += 1;
            }
        }
    }
    Some((found, lod0.meshes.len()))
}

#[path = "mirror_content/census.rs"]
mod census;
#[path = "mirror_content/loaded.rs"]
mod loaded;
#[path = "mirror_content/support.rs"]
mod support;
