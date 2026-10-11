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

/// Vehicle facts needed for playability and display.
struct VehicleFacts {
    name: String,
    info: Info,
    class: Option<&'static str>,
    script_share: bool,
    /// Rail vehicle without a `[sound]` block (R11).
    silent_rail: bool,
    has_driver_cam: bool,
    couple_back: Option<String>,
    cabin: Option<String>,
    refs: Vec<(&'static str, String)>,
}

fn analyze_vehicle(bytes: &[u8], fallback_name: &str) -> VehicleFacts {
    let text = decode_bytes(bytes);
    let blocks = scan_blocks(&text);
    let has = |k: &str| blocks.iter().any(|b| b.keyword.eq_ignore_ascii_case(k));
    let manifest = BusManifest::parse(&text).ok();

    let (manufacturer, model, livery) = manifest
        .as_ref()
        .and_then(|m| m.friendly_name.clone())
        .map_or_else(
            || (String::new(), fallback_name.to_string(), String::new()),
            |f| (f.manufacturer, f.model, f.livery),
        );
    let name = if manufacturer.is_empty() {
        model.clone()
    } else {
        format!("{manufacturer} {model}")
    };
    let class = match first_param(&blocks, "type").as_deref() {
        Some("2") => Some("rail"),
        Some("3") => Some("aircraft"),
        _ => Some("road"),
    };
    let mut refs: Vec<(&'static str, String)> = Vec::new();
    if let Some(m) = &manifest {
        let single = [
            ("model", &m.model_cfg),
            ("sound", &m.sound_cfg),
            ("sound", &m.sound_ai_cfg),
            ("model", &m.paths_cfg),
            ("model", &m.passengercabin_cfg),
        ];
        for (role, v) in single {
            if let Some(v) = v {
                refs.push((role, v.clone()));
            }
        }
        refs.extend(m.scripts.iter().map(|s| ("script", s.clone())));
        refs.extend(m.constfiles.iter().map(|s| ("constfile", s.clone())));
        refs.extend(m.varnamelists.iter().map(|s| ("script", s.clone())));
        refs.extend(m.stringvarnamelists.iter().map(|s| ("script", s.clone())));
    }
    let couple_back = first_param(&blocks, "couple_back");
    let front = first_param(&blocks, "couple_front");
    if let Some(t) = &couple_back {
        refs.push(("trailer_rear", t.clone()));
    }
    if let Some(t) =
        front.filter(|t| t.to_lowercase().ends_with(".bus") || t.to_lowercase().ends_with(".ovh"))
    {
        refs.push(("trailer_front", t));
    }
    VehicleFacts {
        name,
        info: Info::Vehicle {
            manufacturer,
            model,
            livery,
            description: manifest
                .as_ref()
                .map_or(String::new(), |m| m.description.join("\n")),
            mass_tonnes: manifest.as_ref().and_then(|m| m.mass_tonnes).map(f64::from),
        },
        class,
        script_share: has("scriptshare"),
        silent_rail: class == Some("rail")
            && manifest.as_ref().is_some_and(|m| m.sound_cfg.is_none()),
        has_driver_cam: manifest
            .as_ref()
            .is_some_and(|m| !m.driver_cameras.is_empty()),
        couple_back,
        cabin: manifest.and_then(|m| m.passengercabin_cfg),
        refs,
    }
}

/// Applies playability rules R1–R5 plus R9/R10 (`playability-rules.md`). A `.bus` that survives
/// them is playable.
///
/// R9: a vehicle without a complete driver workstation (a driver camera *and* `[drivpos]` in its
/// passenger cabin) cannot be driven (bicycles, fire engines, lorries delivered as `.bus`).
/// R11: a rail vehicle without a `[sound]` block is a passenger-car variant (the NF6D `B` with
/// the dummy plate model and the `Redux` scripts), not a head; the head is `B_User`.
/// R10: vehicles that live in a folder named for AI/KI traffic are traffic, not player vehicles.
fn classify_playable(
    ext: &str,
    rel_path: &str,
    script_share: bool,
    inbound_trailer_target: bool,
    has_full_cab: bool,
    silent_rail: bool,
) -> bool {
    let name = rel_path.rsplit('/').next().unwrap_or(rel_path);
    let r1 = ext == "ovh";
    let r2 = script_share;
    let r3 = inbound_trailer_target && !has_full_cab;
    let r4_r5 = is_trailer_name(name) || is_ai_name(name);
    let r9 = !has_full_cab;
    let r10 = in_ai_folder(rel_path);
    !(r1 || r2 || r3 || r4_r5 || r9 || r10 || silent_rail)
}

/// Whether a directory component of `rel_path` contains the token `ai` or `ki` (split on
/// non-alphanumerics, case-insensitive), e.g. `HOH_KI_Autos`, `AI-Traffic`.
fn in_ai_folder(rel_path: &str) -> bool {
    let mut parts: Vec<&str> = rel_path.split('/').collect();
    parts.pop();
    parts.iter().any(|dir| {
        dir.split(|c: char| !c.is_alphanumeric())
            .any(|t| t.eq_ignore_ascii_case("ai") || t.eq_ignore_ascii_case("ki"))
    })
}

fn is_trailer_name(n: &str) -> bool {
    let n = n.to_lowercase();
    ["_trail", "_trailer", "_heck", "_nachl"]
        .iter()
        .any(|s| stem(&n).ends_with(s))
}

fn is_ai_name(n: &str) -> bool {
    // `ai` / `ki` as a whole token anywhere in the stem: `AI_mb_w123`, `MAN_SD84_KI`,
    // `MB_KI_C2_E5_Solo`, `HHStadtbus_VHH_AI_OMSI2`; never inside a word (`Kaiser`).
    let s = stem(&n.to_lowercase());
    s.split(|c: char| !c.is_alphanumeric())
        .any(|t| t == "ai" || t == "ki")
}

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
mod tests {
    use super::*;

    #[test]
    fn r11_silent_rail_is_not_playable() {
        assert!(!classify_playable(
            "bus",
            "NF6D_B_DT2.bus",
            false,
            false,
            true,
            true
        ));
        assert!(classify_playable(
            "bus",
            "NF6D_B_User.bus",
            false,
            false,
            true,
            false
        ));
    }

    #[test]
    fn name_guards() {
        assert!(is_trailer_name("MAN_GN92_trail.bus") && is_trailer_name("x_heck.bus"));
        assert!(
            is_ai_name("AI_mb_w123.bus")
                && is_ai_name("MAN_SD84_KI.bus")
                && !is_ai_name("MAN_SD77.bus")
        );
    }

    #[test]
    fn playability_rules() {
        assert!(classify_playable(
            "bus",
            "MAN_SD77.bus",
            false,
            false,
            true,
            false
        ));
        assert!(
            !classify_playable("ovh", "BX.ovh", false, false, false, false),
            "R1"
        );
        assert!(
            !classify_playable("bus", "x.bus", true, false, false, false),
            "R2"
        );
        assert!(
            !classify_playable("bus", "NF6D_B.bus", false, true, false, false),
            "R3"
        );
        assert!(
            classify_playable("bus", "NF6D_EEF_V.bus", false, true, true, false),
            "double-ended cab exempt"
        );
        assert!(
            !classify_playable("bus", "MAN_GN92_trail.bus", false, false, false, false),
            "R4"
        );
        assert!(
            !classify_playable("bus", "ai_golf.bus", false, false, true, false),
            "R5"
        );
        assert!(
            !classify_playable(
                "bus",
                "Vehicles/X/MB_KI_C2_E5_Solo.bus",
                false,
                false,
                true,
                false
            ),
            "R5: KI token in the middle"
        );
        assert!(
            !classify_playable(
                "bus",
                "Vehicles/Fahrrad/fahrrad_1.bus",
                false,
                false,
                false,
                false
            ),
            "R9: no driver workstation"
        );
        assert!(
            !classify_playable(
                "bus",
                "Vehicles/HOH_KI_Autos/Truck.bus",
                false,
                false,
                true,
                false
            ),
            "R10: AI folder"
        );
        assert!(
            classify_playable(
                "bus",
                "Vehicles/HOH_Kaiser_Bus/Bus.bus",
                false,
                false,
                true,
                false
            ),
            "'KI' inside a longer token is not an AI folder"
        );
    }

    #[test]
    fn path_normalisation() {
        assert_eq!(
            under("Vehicles/X", "model\\..\\sound\\a.cfg"),
            "Vehicles/X/sound/a.cfg"
        );
    }
}
