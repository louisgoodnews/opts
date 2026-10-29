//! Classifying a vehicle file: its facts, whether it is playable, a trailer or AI-only.

use super::*;

/// Vehicle facts needed for playability and display.
pub(super) struct VehicleFacts {
    pub(super) name: String,
    pub(super) info: Info,
    pub(super) class: Option<&'static str>,
    pub(super) script_share: bool,
    /// Rail vehicle without a `[sound]` block (R11).
    pub(super) silent_rail: bool,
    pub(super) has_driver_cam: bool,
    pub(super) couple_back: Option<String>,
    pub(super) cabin: Option<String>,
    pub(super) refs: Vec<(&'static str, String)>,
}

pub(super) fn analyze_vehicle(bytes: &[u8], fallback_name: &str) -> VehicleFacts {
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
pub(super) fn classify_playable(
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

pub(super) fn is_trailer_name(n: &str) -> bool {
    let n = n.to_lowercase();
    ["_trail", "_trailer", "_heck", "_nachl"]
        .iter()
        .any(|s| stem(&n).ends_with(s))
}

pub(super) fn is_ai_name(n: &str) -> bool {
    // `ai` / `ki` as a whole token anywhere in the stem: `AI_mb_w123`, `MAN_SD84_KI`,
    // `MB_KI_C2_E5_Solo`, `HHStadtbus_VHH_AI_OMSI2`; never inside a word (`Kaiser`).
    let s = stem(&n.to_lowercase());
    s.split(|c: char| !c.is_alphanumeric())
        .any(|t| t == "ai" || t == "ki")
}

fn cabin_has_drivpos(
    dir: &str,
    cabin: &Option<String>,
    by_rel: &HashMap<String, &FileRec>,
    read: &(dyn Fn(&str) -> Option<Vec<u8>> + Sync),
) -> bool {
    cabin.as_ref().is_some_and(|c| {
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
}

pub(super) fn detect_vehicles(
    files: &[FileRec],
    by_rel: &HashMap<String, &FileRec>,
    read: &(dyn Fn(&str) -> Option<Vec<u8>> + Sync),
) -> Vec<EntityDraft> {
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

    let mut trailer_targets: HashSet<String> = HashSet::new();
    let mut primary_by_stem: HashMap<(String, String), String> = HashMap::new();
    for (f, facts) in &analysed {
        if let Some(t) = &facts.couple_back {
            trailer_targets.insert(under(&f.dir, t).to_lowercase());
        }
        if f.ext == "bus" {
            primary_by_stem.insert(
                (f.dir.to_lowercase(), stem(&f.name).to_lowercase()),
                f.rel_path.clone(),
            );
        }
    }

    analysed
        .into_iter()
        .map(|(f, facts)| {
            build_vehicle_draft(f, facts, by_rel, &trailer_targets, &primary_by_stem, read)
        })
        .collect()
}

fn build_vehicle_draft(
    f: &FileRec,
    facts: VehicleFacts,
    by_rel: &HashMap<String, &FileRec>,
    trailer_targets: &HashSet<String>,
    primary_by_stem: &HashMap<(String, String), String>,
    read: &(dyn Fn(&str) -> Option<Vec<u8>> + Sync),
) -> EntityDraft {
    let has_full_cab =
        facts.has_driver_cam && cabin_has_drivpos(&f.dir, &facts.cabin, by_rel, read);
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
    EntityDraft {
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
    }
}
