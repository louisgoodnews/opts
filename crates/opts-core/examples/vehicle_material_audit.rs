//! Material audit of one vehicle model `.cfg` (P-1012): per mesh, how many material slots the
//! `.o3d` declares against the `.cfg` entries, whether every texture the mesh refers to exists
//! (any extension), whether the mesh is scrambled (keyed), and how many UVs lie outside the
//! `[-0.01, 1.01]` square (jitter of undecoded scrambled meshes shows up here).
//! Usage: `vehicle_material_audit <model.cfg> <texture dir>...` (the model dir's `texture` is added).
use std::path::{Path, PathBuf};

use opts_core::formats::o3d::parse;
use opts_core::formats::vehicles::model::ModelConfig;

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for c in rel.split(['\\', '/']).filter(|c| !c.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(c))?;
        cur = hit.path();
    }
    Some(cur)
}

fn texture_exists(dirs: &[PathBuf], name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return true;
    }
    let stem = Path::new(&name.replace('\\', "/"))
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    dirs.iter().any(|d| {
        std::fs::read_dir(d).is_ok_and(|rd| {
            rd.flatten().any(|e| {
                Path::new(&e.file_name())
                    .file_stem()
                    .is_some_and(|s| s.to_string_lossy().to_lowercase() == stem)
            })
        })
    })
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cfg = PathBuf::from(args.next().expect("model .cfg"));
    let mut dirs: Vec<PathBuf> = args.map(PathBuf::from).collect();
    let model_dir = cfg.parent().unwrap().to_path_buf();
    if let Some(t) = find_ci(&model_dir, "texture") {
        dirs.push(t);
    }
    let model = ModelConfig::parse_bytes(&std::fs::read(&cfg).unwrap()).unwrap();
    let lod = model.detail_lod().expect("lod");
    let (mut meshes, mut scrambled, mut slot_mismatch, mut missing_tex, mut jittered) =
        (0, 0, 0, 0, 0);
    println!("mesh | slots(o3d) cfg-entries | scrambled | uv outside | problems");
    for m in &lod.meshes {
        let Some(path) = find_ci(&model_dir, &m.path) else {
            println!("{} | o3d not found", m.path);
            continue;
        };
        let Ok(o) = parse(&std::fs::read(&path).unwrap()) else {
            continue;
        };
        meshes += 1;
        let keyed = o.encryption_key.is_some();
        scrambled += usize::from(keyed);
        let outside = o
            .vertices
            .iter()
            .filter(|v| v.uv.x < -0.01 || v.uv.x > 1.01 || v.uv.y < -0.01 || v.uv.y > 1.01)
            .count();
        let share = outside as f32 / o.vertices.len().max(1) as f32;
        let mut problems = Vec::new();
        // Textures of the o3d slots and the cfg entries.
        for (i, mat) in o.materials.iter().enumerate() {
            for t in &mat.textures {
                if !texture_exists(&dirs, t) {
                    problems.push(format!("slot {i} texture {t:?} missing"));
                }
            }
        }
        for e in &m.materials {
            let mut names = vec![e.texture.clone()];
            names.extend(e.trans_map.clone());
            names.extend(e.night_map.clone());
            names.extend(e.env_map_mask.clone());
            names.extend(e.light_map.as_ref().map(|l| l.texture.clone()));
            names.extend(e.texture_changes.iter().map(|c| c.texture.clone()));
            for t in names {
                if !texture_exists(&dirs, &t) {
                    problems.push(format!("cfg texture {t:?} missing"));
                }
            }
        }
        let used = m
            .materials
            .iter()
            .filter(|e| !e.texture.trim().is_empty() && !e.texture_changes.iter().any(|_| false))
            .count();
        if used > o.materials.len() {
            slot_mismatch += 1;
            problems.push(format!(
                "{used} cfg entries for {} o3d slots",
                o.materials.len()
            ));
        }
        missing_tex += usize::from(problems.iter().any(|p| p.contains("missing")));
        jittered += usize::from(keyed && share > 0.02);
        if !problems.is_empty() || (keyed && share > 0.02) {
            println!(
                "{} | {} {} | {} | {:.0}% | {}",
                m.path,
                o.materials.len(),
                m.materials.len(),
                keyed,
                share * 100.0,
                problems.join("; ")
            );
        }
    }
    println!("meshes {meshes}, scrambled {scrambled}, slot-count mismatches {slot_mismatch}, meshes with missing textures {missing_tex}, scrambled with >2% UV outside the unit square {jittered}");
}
