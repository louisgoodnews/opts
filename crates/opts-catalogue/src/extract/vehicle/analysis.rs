//! Analysis of a vehicle file: its facts and cabin layout.

use super::*;

/// Vehicle facts needed for playability and display.
pub(in crate::extract) struct VehicleFacts {
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

pub(in crate::extract) fn analyze_vehicle(bytes: &[u8], fallback_name: &str) -> VehicleFacts {
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

pub(in crate::extract) fn cabin_has_drivpos(
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
