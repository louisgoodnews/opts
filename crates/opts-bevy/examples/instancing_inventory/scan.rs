//! Scanning helpers of the instancing inventory example: sco loading and part counting.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use opts_bevy::OmsiInstall;
use opts_core::formats::scenery::SceneryObject;

use super::{base_name, mesh_info, MeshInfo, ScoInfo, Totals};

/// Parses a `.sco` and derives what the inventory needs to know about it.
pub(super) fn load_sco(install: &OmsiInstall, sco_path: &Path) -> Option<ScoInfo> {
    let sco = SceneryObject::parse(&std::fs::read(sco_path).ok()?).ok()?;
    let sco_dir = sco_path.parent()?.to_path_buf();
    let model_dir = install
        .find_in(&sco_dir, "model")
        .unwrap_or_else(|| sco_dir.join("model"));
    let meshes = sco
        .meshes
        .iter()
        .map(|m| install.find_in(&model_dir, &m.file))
        .collect();
    let driven = !sco.scripts.is_empty()
        || !sco.lights.is_empty()
        || sco.meshes.iter().any(|m| {
            m.visible.is_some()
                || !m.animations.is_empty()
                || m.materials.iter().any(|x| {
                    x.light_map.is_some() && x.light_map_variable.is_some()
                })
        });
    let lod = sco.meshes.iter().any(|m| m.lod_threshold.is_some());
    let text_textures = sco
        .meshes
        .iter()
        .any(|m| m.materials.iter().any(|x| x.text_texture.is_some()));
    Some(ScoInfo {
        sco,
        meshes,
        dynamic: driven,
        lod_only: lod && !driven,
        text_textures,
    })
}

/// Counts one billboard part for a tree object.
pub(super) fn add_tree(
    t: &mut Totals,
    tile_pairs: &mut HashSet<(u64, usize, String)>,
    first_var: Option<&str>,
    tree_texture: &str,
) {
    // One billboard part per tree: its own mesh (height, width) and material
    // handle today; the content key is the texture.
    t.tree_objects += 1;
    let texture = first_var
        .filter(|s| !s.is_empty())
        .unwrap_or(tree_texture)
        .to_lowercase();
    let content = format!("tree|{texture}");
    t.material_keys.insert(content.clone());
    let mut h = DefaultHasher::new();
    "tree-billboard".hash(&mut h);
    let pair = (h.finish(), 0, content);
    tile_pairs.insert(pair.clone());
    t.pair_keys_flat.insert(pair.clone());
    t.pair_keys.insert(pair);
    t.parts += 1;
    t.parts_flat += 1;
    t.parts_trees += 1;
}

/// Counts the parts of every mesh of one placed object; returns how many it added.
pub(super) fn add_mesh_parts(
    t: &mut Totals,
    tile_pairs: &mut HashSet<(u64, usize, String)>,
    mesh_cache: &mut HashMap<PathBuf, Option<MeshInfo>>,
    info: &ScoInfo,
) -> usize {
    let mut obj_parts = 0;
    for (mi, mesh_def) in info.sco.meshes.iter().enumerate() {
        let Some(path) = info.meshes[mi].clone() else {
            continue;
        };
        let mesh = mesh_cache
            .entry(path.clone())
            .or_insert_with(|| mesh_info(&path));
        let Some(mesh) = mesh.as_ref() else { continue };
        t.mesh_paths.insert(path.clone());
        t.mesh_content.insert(mesh.content);
        let overrides = format!("{:?}", mesh_def.materials);
        let key = (path.clone(), overrides);
        let entry = t.handle_keys.entry(key).or_insert(0);
        *entry = (*entry).max(mesh.slots.len());
        for (si, slot) in mesh.slots.iter().enumerate() {
            // The `[matl]` entry of this slot changes texture, alpha and light map.
            let over = mesh_def.materials.iter().find(|m| {
                base_name(&m.texture) == slot.texture || m.material_index == si
            });
            let content = format!(
                "{}|{:?}|alpha={:?}|lm={:?}/{:?}|tm={}|tt={:?}",
                over.map_or(slot.texture.clone(), |o| {
                    let t = base_name(&o.texture);
                    if t.is_empty() {
                        slot.texture.clone()
                    } else {
                        t
                    }
                }),
                slot.numbers,
                over.and_then(|o| o.alpha_mode),
                over.and_then(|o| o.light_map.as_deref().map(base_name)),
                over.and_then(|o| o.light_map_variable.as_deref()),
                over.is_some_and(|o| o.trans_map),
                over.and_then(|o| o.text_texture),
            );
            t.material_keys.insert(content.clone());
            let pair = (mesh.content, si, content);
            tile_pairs.insert(pair.clone());
            if !info.dynamic && !info.lod_only {
                t.pair_keys_flat.insert(pair.clone());
            }
            t.pair_keys.insert(pair);
            t.parts += 1;
            obj_parts += 1;
            t.triangles += (mesh.triangles / mesh.slots.len().max(1)) as u64;
            if mesh.scrambled {
                t.scrambled_parts += 1;
            }
            if info.text_textures {
                t.parts_text += 1;
            }
            if info.dynamic {
                t.parts_dynamic += 1;
            } else if info.lod_only {
                t.parts_lod_only += 1;
            } else {
                t.parts_flat += 1;
            }
        }
        let _ = mesh.vertices;
    }
    obj_parts
}
