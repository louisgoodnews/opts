//! Map entity detection from `global.cfg` and `ailists.cfg`.

use rayon::prelude::*;

use crate::scan::FileRec;

use super::{decode_bytes, norm, under, EntityDraft, Info, MapManifest};

/// Extracts display metadata and name from an optional map manifest.
fn extract_map_info(manifest: Option<&MapManifest>, folder: &str) -> (String, Info) {
    if let Some(m) = manifest {
        let mut eps: Vec<String> = Vec::new();
        for e in &m.entry_points {
            if !eps.contains(&e.name) {
                eps.push(e.name.clone());
            }
        }
        let friendly = m.friendly_name.clone().unwrap_or_else(|| m.name.clone());
        let display_name = if friendly.is_empty() {
            folder.to_string()
        } else {
            friendly
        };
        (
            display_name.clone(),
            Info::Map {
                friendly_name: display_name,
                description: m.description.join("\n"),
                tile_count: i64::try_from(m.tiles.len()).unwrap_or(0),
                standard_depot: m.standard_depot.clone().unwrap_or_default(),
                years: m
                    .years
                    .map(|(a, b)| format!("{a} – {b}"))
                    .unwrap_or_default(),
                entry_points: eps.join("\n"),
            },
        )
    } else {
        (
            folder.to_string(),
            Info::Map {
                friendly_name: folder.to_string(),
                description: String::new(),
                tile_count: 0,
                standard_depot: String::new(),
                years: String::new(),
                entry_points: String::new(),
            },
        )
    }
}

/// Parses vehicle references from `ailists.cfg`.
fn extract_ailist_refs(bytes: Option<Vec<u8>>) -> Vec<(&'static str, String, Vec<String>)> {
    let mut refs = Vec::new();
    if let Some(b) = bytes {
        for line in decode_bytes(&b).lines() {
            let first = line.split('\t').next().unwrap_or("").trim();
            let l = first.to_lowercase();
            if l.ends_with(".bus") || l.ends_with(".ovh") {
                refs.push(("ai_vehicle", first.to_string(), vec![norm(first)]));
            }
        }
    }
    refs
}

/// Detects map entities from `global.cfg` marker files.
pub(super) fn detect_maps(
    files: &[FileRec],
    read: &(dyn Fn(&str) -> Option<Vec<u8>> + Sync),
) -> Vec<EntityDraft> {
    let map_markers: Vec<&FileRec> = files
        .iter()
        .filter(|f| {
            f.name.eq_ignore_ascii_case("global.cfg")
                && f.rel_path.matches('/').count() == 2
                && f.rel_path.to_lowercase().starts_with("maps/")
        })
        .collect();

    map_markers
        .par_iter()
        .map(|f| {
            let folder = f.dir.rsplit('/').next().unwrap_or("").to_string();
            let manifest = read(&f.rel_path).and_then(|b| MapManifest::parse(&b).ok());
            let (name, info) = extract_map_info(manifest.as_ref(), &folder);
            let refs = extract_ailist_refs(read(&under(&f.dir, "ailists.cfg")));
            EntityDraft {
                category: "map",
                mode: "folder",
                name,
                marker: f.rel_path.clone(),
                dir: f.dir.clone(),
                is_playable: true,
                vehicle_class: None,
                alias_of_marker: None,
                mtime: f.mtime,
                info,
                refs,
            }
        })
        .collect()
}
