//! Detection of vehicles in a scan and building their drafts.

use super::*;

pub(in crate::extract) fn detect_vehicles(
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

pub(in crate::extract) fn build_vehicle_draft(
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
