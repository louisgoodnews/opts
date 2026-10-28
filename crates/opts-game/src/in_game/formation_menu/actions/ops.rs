//! Saving, loading, preset parsing, spawning and validation for the formation editor.

use super::*;

pub(super) fn normal(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

pub(super) fn load_saved(ed: &mut FormationEditor, store: Option<&FormationStore>, i: usize) {
    let (Some(store), Some((id, _, _))) = (store, ed.saved.get(i).cloned()) else {
        return;
    };
    match store.read(&id) {
        Ok(f) => {
            ed.set_formation(f);
            ed.saved_id = Some(id);
        }
        Err(e) => ed.status = format!("{id}: {e}"),
    }
}

pub(super) fn delete_saved(ed: &mut FormationEditor, store: Option<&FormationStore>, i: usize) {
    let (Some(store), Some((id, _, _))) = (store, ed.saved.get(i).cloned()) else {
        return;
    };
    match store.delete(&id) {
        Ok(()) => {
            if ed.saved_id.as_deref() == Some(id.as_str()) {
                ed.saved_id = None;
            }
            ed.saved_dirty = true;
            ed.status = format!("Deleted {id}.");
        }
        Err(e) => ed.status = format!("{id}: {e}"),
    }
}

/// Saves the train: over the file it came from, else as a new file made from its name.
pub(super) fn save(ed: &mut FormationEditor, store: Option<&FormationStore>) {
    let Some(store) = store else {
        ed.status = "There is no folder for saved formations.".into();
        return;
    };
    if ed.formation.is_empty() {
        ed.status = "An empty train is not saved.".into();
        return;
    }
    if ed.formation.name == "Formation" {
        ed.formation.name = ed.formation.suggested_name();
    }
    let result = match ed.saved_id.clone() {
        Some(id) => store.write(&id, &ed.formation).map(|()| id),
        None => store.save_new(&ed.formation),
    };
    match result {
        Ok(id) => {
            ed.status = format!("Saved as {id}.");
            ed.saved_id = Some(id);
            ed.saved_dirty = true;
        }
        Err(e) => ed.status = format!("Not saved: {e}"),
    }
}

pub(super) fn load_preset(ed: &mut FormationEditor, job: Option<&WorldLoadJob>, i: usize) {
    let Some(rel) = ed.presets.get(i).cloned() else {
        return;
    };
    let Some(path) = job.and_then(|j| j.bus_file(&rel)) else {
        ed.status = format!("{rel} not found in the installation.");
        return;
    };
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    match std::fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|b| TrainFormation::from_zug(&b, &name, &rel).map_err(|e| e.to_string()))
    {
        Ok((f, notes)) => {
            ed.set_formation(f); // forgets the saved id
            if !notes.is_empty() {
                ed.status = format!("{rel}: {} line(s) ignored.", notes.len());
            }
        }
        Err(e) => ed.status = format!("{rel}: {e}"),
    }
}

pub(super) fn spawn(
    ed: &mut FormationEditor,
    menu: &mut VehicleMenu,
    job: Option<&WorldLoadJob>,
    registry: &mut SessionVehicles,
) {
    let Some(job) = job else { return };
    let Some(entry) = menu.entry() else {
        ed.status = "Choose an entry point.".into();
        return;
    };
    let resolved = match resolve(&ed.formation, job) {
        Ok(r) => r,
        Err(issues) => {
            ed.status = issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            return;
        }
    };
    if matches!(ed.formation.source, FormationSource::Custom) && ed.formation.name == "Formation" {
        ed.formation.name = ed.formation.suggested_name();
    }
    queue_spawn(registry, &ed.formation, &resolved, EntrySel::Id(entry));
    menu.close();
}

/// Checks the formation and totals its files and mass after a change.
pub(in super::super) fn refresh_validation(
    mut ed: ResMut<FormationEditor>,
    job: Option<Res<WorldLoadJob>>,
) {
    if ed.validated == ed.revision {
        return;
    }
    let Some(job) = job else { return };
    let lookup = JobLookup(&job);
    ed.issues = ed.formation.validate(&lookup);
    let mut parts = 0;
    let mut mass = 0.0;
    for file in ed.formation.file_refs() {
        parts += lookup.info(file).map_or(0, |i| i.parts);
        let key = normal(file);
        mass += ed
            .cars
            .iter()
            .find(|c| normal(&c.rel) == key)
            .and_then(|c| c.mass_t)
            .unwrap_or(0.0);
    }
    ed.summary = Summary {
        parts,
        mass_t: mass,
    };
    ed.validated = ed.revision;
}
