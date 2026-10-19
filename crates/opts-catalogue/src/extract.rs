//! Entity detection, display metadata and reference extraction.
//!
//! Marker rules follow `catalogue-schema.md` (Stage 2/Part C) and vehicle playability follows
//! `playability-rules.md` (rules R1–R8).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use opts_core::formats::map::MapManifest;
use opts_core::formats::scenery::{SceneryObject, SplineProfile};
use opts_core::formats::text::{decode_bytes, scan_blocks};
use opts_core::formats::vehicles::BusManifest;
use rayon::prelude::*;

use crate::scan::{DirRec, FileRec};

/// Display metadata attached to an entity.
#[derive(Debug, Clone)]
pub enum Info {
    /// No extra metadata.
    None,
    /// Map metadata from `global.cfg`.
    Map {
        /// Display name.
        friendly_name: String,
        /// Description text.
        description: String,
        /// Number of `[map]` tile entries.
        tile_count: i64,
        /// Default depot.
        standard_depot: String,
        /// Year range text.
        years: String,
        /// Entry point names (newline separated, de-duplicated).
        entry_points: String,
    },
    /// Vehicle metadata from `.bus` / `.ovh`.
    Vehicle {
        /// Manufacturer.
        manufacturer: String,
        /// Model.
        model: String,
        /// Default livery.
        livery: String,
        /// Description text.
        description: String,
        /// Curb mass.
        mass_tonnes: Option<f64>,
    },
}

/// An entity found by the marker rules, before it is written to the database.
#[derive(Debug, Clone)]
pub struct EntityDraft {
    /// One of the 16 categories.
    pub category: &'static str,
    /// `"folder"` or `"file"`.
    pub mode: &'static str,
    /// Display name.
    pub name: String,
    /// Marker file's relative path.
    pub marker: String,
    /// Entity directory (relative path).
    pub dir: String,
    /// Player-selectable vehicle.
    pub is_playable: bool,
    /// `road` / `rail` / `aircraft` for vehicles.
    pub vehicle_class: Option<&'static str>,
    /// Marker of the primary entity if this is an alias twin.
    pub alias_of_marker: Option<String>,
    /// Marker mtime.
    pub mtime: i64,
    /// Display metadata.
    pub info: Info,
    /// `(role, raw_reference, root-relative candidate paths in search order)`.
    pub refs: Vec<(&'static str, String, Vec<String>)>,
}

fn norm(rel: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for c in rel.split(['\\', '/']) {
        match c.trim() {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    parts.join("/")
}

fn under(dir: &str, rel: &str) -> String {
    norm(&format!("{dir}/{rel}"))
}

fn first_param(blocks: &[opts_core::formats::text::OmsiBlock], key: &str) -> Option<String> {
    blocks
        .iter()
        .find(|b| b.keyword.eq_ignore_ascii_case(key))
        .and_then(|b| b.parameters.first())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn stem(name: &str) -> String {
    name.rsplit_once('.').map_or(name, |(s, _)| s).to_string()
}

mod vehicle;
use vehicle::{analyze_vehicle, classify_playable, VehicleFacts};

/// File-entity categories: `(root folder, extensions, category)`.
const FILE_ENTITIES: &[(&str, &[&str], &str)] = &[
    ("Sceneryobjects", &["sco"], "sceneryobject"),
    ("Splines", &["sli"], "spline"),
    ("Fonts", &["oft"], "font"),
    ("Trains", &["zug"], "train_consist"),
    ("Situations", &["osn"], "situation"),
    ("Weather", &["owt"], "weather"),
    ("option_presets", &["oop"], "option_preset"),
    ("Languages", &["olf"], "language"),
    ("Inputs", &["kyb"], "input_map"),
    ("GUI", &["bmp"], "gui_skin"),
    ("Drivers", &["odr"], "driver"),
    ("Humans", &["hum"], "human"),
    ("TicketPacks", &["otp"], "ticketpack"),
];

/// Candidate locations of a texture named by an object in `dir`: the object's own `texture/`
/// folder, the object folder, then the global `Texture/` folder; each tried with the exact name
/// and then with the other image extensions (material names often omit or mis-state them).
fn texture_candidates(dir: &str, name: &str) -> Vec<String> {
    let n = norm(name);
    let stem = n
        .rsplit_once('.')
        .map_or(n.as_str(), |(s, _)| s)
        .to_string();
    let bases = [
        format!("{dir}/texture"),
        dir.to_string(),
        "Texture".to_string(),
    ];
    let mut out = Vec::new();
    for base in &bases {
        out.push(norm(&format!("{base}/{n}")));
    }
    for base in &bases {
        for ext in ["dds", "bmp", "tga", "png", "jpg"] {
            out.push(norm(&format!("{base}/{stem}.{ext}")));
        }
    }
    out
}

fn scenery_refs(dir: &str, sco: &SceneryObject) -> Vec<(&'static str, String, Vec<String>)> {
    let mut refs = Vec::new();
    for mesh in &sco.meshes {
        refs.push((
            "model",
            mesh.file.clone(),
            vec![
                under(&format!("{dir}/model"), &mesh.file),
                under(dir, &mesh.file),
            ],
        ));
        for m in &mesh.materials {
            let named = [Some(&m.texture), m.light_map.as_ref(), m.night_map.as_ref()]
                .into_iter()
                .flatten()
                .chain(m.env_map.as_ref().map(|(t, _)| t));
            for t in named.filter(|t| !t.is_empty()) {
                refs.push(("texture", t.clone(), texture_candidates(dir, t)));
            }
        }
    }
    for c in &sco.collision_meshes {
        refs.push((
            "model",
            c.clone(),
            vec![under(&format!("{dir}/model"), c), under(dir, c)],
        ));
    }
    for h in &sco.spline_helpers {
        refs.push((
            "spline",
            h.profile_file.clone(),
            vec![norm(&h.profile_file), under(dir, &h.profile_file)],
        ));
    }
    for (role, list) in [
        ("script", &sco.scripts),
        ("script", &sco.varnamelists),
        ("script", &sco.stringvarnamelists),
        ("constfile", &sco.constfiles),
    ] {
        refs.extend(list.iter().map(|f| (role, f.clone(), vec![under(dir, f)])));
    }
    if let Some(s) = &sco.sound {
        refs.push(("sound", s.clone(), vec![under(dir, s)]));
    }
    if let Some(t) = &sco.tree {
        refs.push((
            "texture",
            t.texture.clone(),
            texture_candidates(dir, &t.texture),
        ));
    }
    refs
}

fn spline_refs(dir: &str, sli: &SplineProfile) -> Vec<(&'static str, String, Vec<String>)> {
    sli.textures
        .iter()
        .filter(|t| !t.file.is_empty())
        .map(|t| ("texture", t.file.clone(), texture_candidates(dir, &t.file)))
        .collect()
}

fn friendly_name(bytes: &[u8]) -> Option<String> {
    let blocks = scan_blocks(&decode_bytes(bytes));
    first_param(&blocks, "friendlyname")
}

/// Detects all entities and extracts their metadata. `root` is read for marker contents.
#[must_use]
pub fn detect_entities(root: &Path, files: &[FileRec], dirs: &[DirRec]) -> Vec<EntityDraft> {
    let by_rel: HashMap<String, &FileRec> = files
        .iter()
        .map(|f| (f.rel_path.to_lowercase(), f))
        .collect();
    let read = |rel: &str| fs::read(root.join(rel)).ok();
    let mut out: Vec<EntityDraft> = Vec::new();

    // --- Vehicles ---------------------------------------------------------------------
    let vehicle_files: Vec<&FileRec> = files
        .iter()
        .filter(|f| {
            !f.is_backup
                && matches!(f.ext.as_str(), "bus" | "ovh")
                && f.rel_path.starts_with("Vehicles/")
        })
        .collect();
    let analysed: Vec<(&FileRec, VehicleFacts)> = vehicle_files
        .par_iter()
        .map(|f| {
            let facts = read(&f.rel_path).map_or_else(
                || analyze_vehicle(&[], &stem(&f.name)),
                |b| analyze_vehicle(&b, &stem(&f.name)),
            );
            (*f, facts)
        })
        .collect();

    // Inbound [couple_back] targets (R3).
    let mut trailer_targets: HashSet<String> = HashSet::new();
    for (f, facts) in &analysed {
        if let Some(t) = &facts.couple_back {
            trailer_targets.insert(under(&f.dir, t).to_lowercase());
        }
    }
    // Driver workstation also needs `[drivpos]` in the passenger cabin.
    let cabin_has_drivpos = |dir: &str, cabin: &Option<String>| -> bool {
        cabin.as_ref().is_some_and(|c| {
            // The path in the `.bus` rarely matches the on-disk case; resolve it via the index.
            let wanted = under(dir, c).to_lowercase();
            by_rel
                .get(&wanted)
                .and_then(|f| read(&f.rel_path))
                .is_some_and(|b| {
                    scan_blocks(&decode_bytes(&b))
                        .iter()
                        .any(|bl| bl.keyword.eq_ignore_ascii_case("drivpos"))
                })
        })
    };

    let mut primary_by_stem: HashMap<(String, String), String> = HashMap::new();
    for (f, _) in &analysed {
        if f.ext == "bus" {
            primary_by_stem.insert(
                (f.dir.to_lowercase(), stem(&f.name).to_lowercase()),
                f.rel_path.clone(),
            );
        }
    }
    for (f, facts) in analysed {
        let has_full_cab = facts.has_driver_cam && cabin_has_drivpos(&f.dir, &facts.cabin);
        let is_playable = classify_playable(
            &f.ext,
            &f.rel_path,
            facts.script_share,
            trailer_targets.contains(&f.rel_path.to_lowercase()),
            has_full_cab,
            facts.silent_rail,
        );
        let alias_of_marker = (f.ext == "ovh")
            .then(|| {
                primary_by_stem
                    .get(&(f.dir.to_lowercase(), stem(&f.name).to_lowercase()))
                    .cloned()
            })
            .flatten();
        let refs = facts
            .refs
            .iter()
            .map(|(role, raw)| (*role, raw.clone(), vec![under(&f.dir, raw)]))
            .collect();
        out.push(EntityDraft {
            category: "vehicle",
            mode: "folder",
            name: facts.name,
            marker: f.rel_path.clone(),
            dir: f.dir.clone(),
            is_playable,
            vehicle_class: facts.class,
            alias_of_marker,
            mtime: f.mtime,
            info: facts.info,
            refs,
        });
    }

    // --- Maps ---------------------------------------------------------------------------
    let map_markers: Vec<&FileRec> = files
        .iter()
        .filter(|f| {
            f.name.eq_ignore_ascii_case("global.cfg")
                && f.rel_path.matches('/').count() == 2
                && f.rel_path.to_lowercase().starts_with("maps/")
        })
        .collect();
    let maps: Vec<EntityDraft> = map_markers
        .par_iter()
        .map(|f| {
            let folder = f.dir.rsplit('/').next().unwrap_or("").to_string();
            let manifest = read(&f.rel_path).and_then(|b| MapManifest::parse(&b).ok());
            let mut eps: Vec<String> = Vec::new();
            let (name, info) = if let Some(m) = &manifest {
                for e in &m.entry_points {
                    if !eps.contains(&e.name) {
                        eps.push(e.name.clone());
                    }
                }
                let friendly = m.friendly_name.clone().unwrap_or_else(|| m.name.clone());
                (
                    if friendly.is_empty() {
                        folder.clone()
                    } else {
                        friendly.clone()
                    },
                    Info::Map {
                        friendly_name: if friendly.is_empty() {
                            folder.clone()
                        } else {
                            friendly
                        },
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
                    folder.clone(),
                    Info::Map {
                        friendly_name: folder.clone(),
                        description: String::new(),
                        tile_count: 0,
                        standard_depot: String::new(),
                        years: String::new(),
                        entry_points: String::new(),
                    },
                )
            };
            // Vehicles used by the map's AI lists.
            let mut refs = Vec::new();
            if let Some(b) = read(&under(&f.dir, "ailists.cfg")) {
                for line in decode_bytes(&b).lines() {
                    let first = line.split('\t').next().unwrap_or("").trim();
                    let l = first.to_lowercase();
                    if l.ends_with(".bus") || l.ends_with(".ovh") {
                        refs.push(("ai_vehicle", first.to_string(), vec![norm(first)]));
                    }
                }
            }
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
        .collect();
    out.extend(maps);

    // --- Money packs (one per currency folder) --------------------------------------------
    let mut seen_money: HashSet<String> = HashSet::new();
    for f in files
        .iter()
        .filter(|f| !f.is_backup && f.ext == "cfg" && f.rel_path.starts_with("Money/"))
    {
        if f.rel_path.matches('/').count() == 2 && seen_money.insert(f.dir.clone()) {
            out.push(simple(
                "money",
                "folder",
                f,
                f.dir.rsplit('/').next().unwrap_or("").to_string(),
            ));
        }
    }

    // --- File entities ------------------------------------------------------------------------
    let file_entities: Vec<EntityDraft> = files
        .par_iter()
        .filter(|f| !f.is_backup)
        .filter_map(|f| {
            let top = f.rel_path.split('/').next()?;
            let (_, _, cat) = FILE_ENTITIES.iter().find(|(root_dir, exts, _)| {
                top.eq_ignore_ascii_case(root_dir) && exts.contains(&f.ext.as_str())
            })?;
            let bytes = if matches!(*cat, "sceneryobject" | "spline" | "human") {
                read(&f.rel_path)
            } else {
                None
            };
            let mut refs = Vec::new();
            let name = match (*cat, &bytes) {
                ("sceneryobject", Some(b)) => match SceneryObject::parse(b) {
                    Ok(sco) => {
                        refs = scenery_refs(&f.dir, &sco);
                        sco.friendly_name
                            .clone()
                            .filter(|n| !n.trim().is_empty())
                            .unwrap_or_else(|| stem(&f.name))
                    }
                    Err(_) => stem(&f.name),
                },
                ("spline", Some(b)) => {
                    if let Ok(sli) = SplineProfile::parse(b) {
                        refs = spline_refs(&f.dir, &sli);
                    }
                    stem(&f.name)
                }
                ("human", Some(b)) => friendly_name(b).unwrap_or_else(|| stem(&f.name)),
                _ => stem(&f.name),
            };
            let mut draft = simple(cat, "file", f, name);
            draft.refs = refs;
            Some(draft)
        })
        .collect();
    out.extend(file_entities);
    let _ = (by_rel, dirs);
    out
}

fn simple(category: &'static str, mode: &'static str, f: &FileRec, name: String) -> EntityDraft {
    EntityDraft {
        category,
        mode,
        name,
        marker: f.rel_path.clone(),
        dir: f.dir.clone(),
        is_playable: true,
        vehicle_class: None,
        alias_of_marker: None,
        mtime: f.mtime,
        info: Info::None,
        refs: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
