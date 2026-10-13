//! Saving the running session, the list and the preflight check of a save. The UI, the quick keys,
//! the autosave and the control protocol all call these functions.

use bevy::prelude::*;

use super::container::{clean_name, Fingerprint, SaveError, SaveFile, SaveMeta};
use super::registry::{carry_over_foreign, SaveRegistry};
use super::store::{SaveEntry, SaveStore};
use super::{fingerprint, PlayClock, SaveStorage};
use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldLoadJob;
use crate::in_game::types::{InGameSessionInfo, SimulationLoadState};
use crate::session_vehicles::{Lifecycle, SessionVehicles};

/// What to save and under which name.
#[derive(Debug, Clone, Default)]
pub struct SaveRequest {
    /// The name shown in the list (cleaned before use).
    pub name: String,
    /// Write to this save (it is overwritten); `None` makes a new save, or, when `overwrite` is
    /// set, replaces the save with the same name.
    pub id: Option<String>,
    /// Replace a save that has the same name instead of refusing ([`SaveError::Exists`]).
    pub overwrite: bool,
}

/// The result of a successful save.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    /// File stem.
    pub id: String,
    /// Display name.
    pub name: String,
    /// An existing save was replaced.
    pub overwritten: bool,
}

/// The configured store.
///
/// # Errors
/// No user data directory is known.
pub fn store(world: &World) -> Result<SaveStore, SaveError> {
    world
        .get_resource::<SaveStorage>()
        .and_then(|s| s.0.clone())
        .ok_or_else(|| SaveError::Io("no data directory for saves".to_owned()))
}

/// Whether a drive is running and loaded (the only time a save makes sense).
#[must_use]
pub fn session_running(world: &World) -> bool {
    world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| *s.get() == AppState::InGame)
        && world
            .get_resource::<SimulationLoadState>()
            .is_some_and(|s| s.is_complete)
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

/// The fingerprint of the content of the running session.
fn session_fingerprint(world: &World) -> Fingerprint {
    let mut f = Fingerprint::default();
    if let Some(job) = world.get_resource::<WorldLoadJob>() {
        let dir = job.map_dir();
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if let Some(hash) = fingerprint::map(dir) {
            f.map = Some((name, hash));
        }
    }
    if let Some(reg) = world.get_resource::<SessionVehicles>() {
        for r in reg
            .records()
            .iter()
            .filter(|r| matches!(r.lifecycle, Lifecycle::Alive | Lifecycle::Spawning))
        {
            let path = std::path::Path::new(&r.bus_file);
            if let Some(hash) = fingerprint::vehicle(path) {
                let key = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| r.bus_file.clone());
                f.vehicles.insert(key, hash);
            }
        }
    }
    f
}

/// Saves the running session.
///
/// # Errors
/// [`SaveError::NoSession`] outside a loaded drive, [`SaveError::Exists`] when the name is taken
/// and `overwrite` is not set, [`SaveError::Capture`] when the `session` section cannot be
/// captured, and the store's errors.
pub fn save_session(world: &mut World, request: &SaveRequest) -> Result<Saved, SaveError> {
    if !session_running(world) {
        return Err(SaveError::NoSession(
            "there is no running drive to save".to_owned(),
        ));
    }
    let name = clean_name(&request.name)?;
    let store = store(world)?;
    let (id, existing) = match &request.id {
        Some(id) => (id.clone(), store.read(id).ok()),
        None => match store.find(&name).filter(|_| request.overwrite) {
            Some(id) => (id.clone(), store.read(&id).ok()),
            None => {
                if let Some(id) = store.find(&name) {
                    return Err(SaveError::Exists(id));
                }
                (store.unique_id(&name), None)
            }
        },
    };
    let sections = world.resource::<SaveRegistry>().all().to_vec();
    let (blobs, failures) = SaveRegistry::capture_all(&sections, world);
    for (section, message) in failures {
        let required = sections.iter().any(|s| s.id == section && s.required);
        if required {
            return Err(SaveError::Capture { section, message });
        }
        tracing::warn!(%section, "section not saved: {message}");
    }
    let session = world.resource::<InGameSessionInfo>();
    let vehicles: Vec<String> = world
        .resource::<SessionVehicles>()
        .records()
        .iter()
        .filter(|r| matches!(r.lifecycle, Lifecycle::Alive | Lifecycle::Spawning))
        .map(|r| r.name.clone())
        .collect();
    let now = now_unix();
    let mut file = SaveFile::new(SaveMeta {
        name: name.clone(),
        created_unix: existing
            .as_ref()
            .map_or(now, |l| l.file.meta.created_unix),
        saved_unix: now,
        play_time_s: world
            .get_resource::<PlayClock>()
            .map_or(0.0, |c| c.played_s),
        map: session.map_name.clone(),
        vehicles,
        thumbnail: None,
    });
    file.fingerprint = session_fingerprint(world);
    file.sections = blobs;
    if let Some(old) = &existing {
        carry_over_foreign(&sections, &old.file, &mut file);
    }
    store.write(&id, &file)?;
    tracing::info!(%id, %name, sections = file.sections.len(), "session saved");
    Ok(Saved {
        id,
        name,
        overwritten: existing.is_some(),
    })
}

/// The list of saves of the configured store.
#[must_use]
pub fn list(world: &World) -> Vec<SaveEntry> {
    store(world).map(|s| s.list()).unwrap_or_default()
}
