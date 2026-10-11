//! Which script variables do the models of articulated vehicles (`[couple_back]`/`[couple_front]`)
//! use, and how many of their meshes are skinned (`[smoothskin]`, bellows)? Usage: `-- <OMSI root>`.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use opts_core::formats::vehicles::model::ModelConfig;
use opts_core::formats::vehicles::BusManifest;

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for part in rel.split(['\\', '/']).filter(|p| !p.is_empty()) {
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(part))?;
        cur = hit.path();
    }
    Some(cur)
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI root"));
    let (mut vehicles, mut skinned_meshes, mut bone_meshes) = (0, 0, 0);
    let mut anim: BTreeMap<String, usize> = BTreeMap::new();
    let mut chars: BTreeMap<String, usize> = BTreeMap::new();
    for d in std::fs::read_dir(root.join("Vehicles")).unwrap().flatten() {
        let Ok(rd) = std::fs::read_dir(d.path()) else {
            continue;
        };
        for f in rd.flatten() {
            let n = f.file_name().to_string_lossy().to_lowercase();
            if !(n.ends_with(".bus") || n.ends_with(".ovh")) {
                continue;
            }
            let Some(bytes) = std::fs::read(f.path()).ok() else {
                continue;
            };
            let Ok(m) = BusManifest::parse_bytes(&bytes) else {
                continue;
            };
            if m.couple_back.is_none() && m.coupling_front.is_none() {
                continue;
            }
            let text = opts_core::formats::text::decode_bytes(&bytes);
            for b in opts_core::formats::text::scan_blocks(&text) {
                if b.keyword == "coupling_front_character" {
                    *chars.entry(b.parameters.join(" | ")).or_default() += 1;
                }
            }
            let Some(model) = m
                .model_cfg
                .as_deref()
                .and_then(|c| find_ci(&d.path(), c))
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| ModelConfig::parse_bytes(&b).ok())
            else {
                continue;
            };
            vehicles += 1;
            let Some(lod) = model
                .lods
                .iter()
                .find(|l| l.threshold == 0.0)
                .or(model.lods.first())
            else {
                continue;
            };
            for mesh in &lod.meshes {
                skinned_meshes += usize::from(mesh.smoothskin);
                bone_meshes += usize::from(!mesh.bones.is_empty());
                for a in &mesh.anims {
                    for x in a.anim_rot.iter().chain(a.anim_trans.iter()) {
                        *anim.entry(x.variable.to_lowercase()).or_default() += 1;
                    }
                }
            }
        }
    }
    println!("{vehicles} articulated vehicle models; {skinned_meshes} [smoothskin] meshes, {bone_meshes} with [setbone]");
    println!("coupling_front_character: {chars:?}");
    let mut v: Vec<_> = anim.into_iter().collect();
    v.sort_by_key(|e| std::cmp::Reverse(e.1));
    let words = [
        "angle", "winkel", "gelenk", "knick", "coupl", "art", "joint", "bend", "bone", "balg",
        "kupp", "alpha", "beta", "turn", "dreh",
    ];
    let hits: Vec<_> = v
        .iter()
        .filter(|(n, _)| words.iter().any(|w| n.contains(w)))
        .take(30)
        .collect();
    println!("variables with an articulation-like name: {hits:?}");
    println!("most used animation variables: {:?}", &v[..v.len().min(25)]);
}
