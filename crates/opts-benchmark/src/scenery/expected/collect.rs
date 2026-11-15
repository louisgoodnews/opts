//! Collecting the expected triangles of one object from its meshes.

use super::*;

/// Builds the expected triangles of `sco` (file at `sco_path`).
#[must_use]
pub fn build(install: &OmsiInstall, sco_path: &Path, sco: &SceneryObject) -> Expected {
    let mut out = Expected::default();
    if let Some(tree) = &sco.tree {
        let h = (tree.min_height + tree.max_height) * 0.5;
        let w = h * (tree.min_width_ratio + tree.max_width_ratio) * 0.5;
        out.tris = tree_billboard(h, w);
        return out;
    }
    if sco.is_editor_only() || sco.carpark_p || sco.model_cfg.is_some() {
        return out;
    }
    let sco_dir = sco_path.parent().unwrap_or(Path::new("."));
    let model_dir = install
        .find_in(sco_dir, "model")
        .unwrap_or_else(|| sco_dir.join("model"));
    let lods: Vec<f32> = sco.meshes.iter().filter_map(|m| m.lod_threshold).collect();
    let top_lod = lods
        .iter()
        .copied()
        .fold(None, |a: Option<f32>, b| Some(a.map_or(b, |a| a.max(b))));
    let mut distinct = lods.clone();
    distinct.sort_by(f32::total_cmp);
    distinct.dedup();
    out.lod_groups = distinct.len();
    for (mesh_index, mesh) in sco.meshes.iter().enumerate() {
        if !sco.mesh_in_game(mesh_index) {
            continue;
        }
        let Some(path) = install.find_in(&model_dir, &mesh.file) else {
            push_unique(&mut out.categories, "F-MESH-MISSING");
            // The loader gives up on the whole object at the first missing mesh.
            out.tris.clear();
            return out;
        };
        let base = mesh.visible.is_none() && mesh.lod_threshold.is_none_or(|l| Some(l) == top_lod);
        let models = match read_models(&path, &mut out.keyed) {
            Ok(m) => m,
            Err(()) => {
                push_unique(&mut out.categories, "F-MESH-PARSE");
                continue;
            }
        };
        let overrides = overrides(sco, mesh);
        for model in &models {
            add_model(&mut out.tris, model, mesh_index, &overrides, base);
        }
    }
    out
}

fn push_unique(v: &mut Vec<String>, s: &str) {
    if !v.iter().any(|x| x == s) {
        v.push(s.to_string());
    }
}
