//! Restoring a save: the `prepare` functions run, the session starts through the normal loading
//! screen, and the sections are applied in stages as the session becomes ready.
//!
//! 1. [`begin_restore`] reads and checks the file, runs every section's `prepare` (they write the
//!    choices the loading pipeline reads) and enters the session.
//! 2. When the HUD is up, the [`Stage::World`] sections run (they queue the saved vehicles).
//! 3. The spawner rebuilds the vehicles; each receives its state once its scripts have started.
//! 4. When no vehicle is spawning and none waits for its state, the [`Stage::Settled`] sections run
//!    (active vehicle, camera, counters, paused state) and the restore is done.

use bevy::prelude::*;

use super::container::SaveError;
use super::ops::store;
use super::registry::{plan_restore, preflight, CheckEnv, SaveRegistry};
use crate::app_state::AppState;

mod stages;
mod types;

pub use stages::{drive_restore, forget_pending};
pub use types::{
    LastRestore, PendingRestore, RestoreError, RestoreOutcome, RestoreStarted, QUIET_FRAMES,
    SETTLE_TIMEOUT_S,
};
use types::issue_text;

/// Starts restoring save `id`: checks it, prepares the session and enters it. The caller is on
/// Home or inside a running drive (that drive is left without asking: the UI asks first).
///
/// # Errors
/// [`RestoreError::Save`] for an unreadable, damaged or newer file or one without a required
/// section, [`RestoreError::Blocked`] when the map or every vehicle is missing.
pub fn begin_restore(world: &mut World, id: &str) -> Result<RestoreStarted, RestoreError> {
    let loaded = store(world)?.read(id)?;
    let sections = world.resource::<SaveRegistry>().all().to_vec();
    let plan = plan_restore(&sections, &loaded.file);
    for required in sections.iter().filter(|s| s.required) {
        if !plan.steps.iter().any(|(s, _)| s.id == required.id) {
            return Err(SaveError::Corrupt(format!(
                "the required section `{}` is missing or unusable",
                required.id
            ))
            .into());
        }
    }
    let root = world
        .get_resource::<crate::in_game::omsi_world::WorldLoadJob>()
        .map(|j| j.omsi_root().to_path_buf())
        .or_else(crate::omsi_root::known_omsi_root);
    let issues = preflight(
        &sections,
        &loaded.file,
        &CheckEnv {
            root: root.as_deref(),
            fingerprint: &loaded.file.fingerprint,
        },
    );
    if issues.iter().any(|i| i.blocking) {
        return Err(RestoreError::Blocked(issues));
    }
    let mut warnings: Vec<String> = plan.warnings.clone();
    warnings.extend(issues.iter().map(issue_text));
    for (section, data) in plan.prepare_steps().into_iter().map(|s| (&s.0, &s.1)) {
        if let Some(prepare) = section.prepare {
            prepare(world, data)
                .map_err(|e| SaveError::Corrupt(format!("section `{}`: {e}", section.id)))?;
        }
    }
    let vehicle_label = loaded
        .file
        .meta
        .vehicles
        .first()
        .cloned()
        .unwrap_or_default();
    let started = RestoreStarted {
        id: id.to_owned(),
        name: loaded.file.meta.name.clone(),
        unknown_sections: plan.unknown.clone(),
        missing_sections: plan.missing.clone(),
        warnings: warnings.clone(),
    };
    world.insert_resource(PendingRestore {
        id: id.to_owned(),
        name: loaded.file.meta.name,
        vehicle_label,
        plan,
        world_applied: false,
        vehicles_applied: false,
        quiet_frames: 0,
        waited_s: 0.0,
        warnings,
        from_backup: loaded.from_backup,
    });
    world.insert_resource(LastRestore(None));
    // A running session is left and entered again (the same state: the transition re-runs).
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    tracing::info!(%id, "restoring a save");
    Ok(started)
}
