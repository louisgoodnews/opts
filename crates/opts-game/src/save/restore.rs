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
use opts_gui::components::toast::ToastSeverity;

use super::container::SaveError;
use super::ops::store;
use super::registry::{plan_restore, preflight, CheckEnv, Issue, RestorePlan, SaveRegistry, Stage};
use crate::app_state::AppState;
use crate::in_game::omsi_world::SpawnVehicleRequest;
use crate::in_game::types::{LoadStage, SimulationLoadState};
use crate::session_vehicles::{Lifecycle, SessionVehicles};
use crate::shell::keys;
use crate::shell::notify::NotifyExt;

/// How long a restore waits for its vehicles before it settles without them (seconds).
pub const SETTLE_TIMEOUT_S: f32 = 120.0;
/// Frames without a spawning vehicle before the saved vehicle state is applied.
pub const QUIET_FRAMES: u32 = 10;

/// Why a load was refused or failed.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreError {
    /// The file could not be read or is not usable.
    Save(SaveError),
    /// Content the save needs is missing from the installation.
    Blocked(Vec<Issue>),
}

impl RestoreError {
    /// A stable word for logs and the control protocol.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Save(e) => e.kind(),
            Self::Blocked(_) => "missing_content",
        }
    }
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Save(e) => e.fmt(f),
            Self::Blocked(issues) => {
                let list: Vec<&str> = issues
                    .iter()
                    .filter(|i| i.blocking)
                    .map(|i| i.subject.as_str())
                    .collect();
                write!(f, "missing content: {}", list.join(", "))
            }
        }
    }
}

impl From<SaveError> for RestoreError {
    fn from(e: SaveError) -> Self {
        Self::Save(e)
    }
}

/// A restore in progress.
#[derive(Resource, Debug, Clone)]
pub struct PendingRestore {
    /// The save's id.
    pub id: String,
    /// The save's name.
    pub name: String,
    /// Name of the first saved vehicle (the loading screen's vehicle line); empty without one.
    pub vehicle_label: String,
    plan: RestorePlan,
    world_applied: bool,
    vehicles_applied: bool,
    quiet_frames: u32,
    waited_s: f32,
    /// Problems so far (sections that could not be applied, newer or unknown sections).
    pub warnings: Vec<String>,
    /// The main file was damaged and the backup was used.
    pub from_backup: bool,
}

/// How the last restore ended.
#[derive(Debug, Clone, PartialEq)]
pub enum RestoreOutcome {
    /// Everything that could be restored was.
    Done {
        /// The save.
        id: String,
        /// Problems on the way.
        warnings: Vec<String>,
    },
    /// The session could not be loaded (the loading screen shows why).
    Failed {
        /// The save.
        id: String,
    },
}

/// The outcome of the most recent restore.
#[derive(Resource, Debug, Clone, Default)]
pub struct LastRestore(pub Option<RestoreOutcome>);

/// What a started restore reports.
#[derive(Debug, Clone, PartialEq)]
pub struct RestoreStarted {
    /// The save's id.
    pub id: String,
    /// The save's name.
    pub name: String,
    /// Sections kept but not understood by this build.
    pub unknown_sections: Vec<String>,
    /// Sections the save lacks (they keep their defaults).
    pub missing_sections: Vec<String>,
    /// Content differences that did not block the load.
    pub warnings: Vec<String>,
}

fn issue_text(i: &Issue) -> String {
    format!("{:?}: {}", i.kind, i.subject)
}

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

fn run_stage(world: &mut World, stage: Stage) {
    let steps: Vec<_> = world
        .resource::<PendingRestore>()
        .plan
        .steps
        .iter()
        .filter(|(s, _)| s.stage == stage && s.apply.is_some())
        .cloned()
        .collect();
    for (section, data) in steps {
        let Some(apply) = section.apply else { continue };
        if let Err(e) = apply(world, &data) {
            let message = format!("section `{}` could not be restored: {e}", section.id);
            tracing::warn!("{message}");
            world
                .resource_mut::<PendingRestore>()
                .warnings
                .push(message);
        }
    }
}

fn toast(world: &mut World, key: &'static str, args: Vec<(&'static str, String)>) {
    world.commands().notify(ToastSeverity::Info, key, args);
    world.flush();
}

/// Drives a restore forward each frame while a session is up (see the module documentation).
pub fn drive_restore(world: &mut World) {
    if !world.contains_resource::<PendingRestore>() {
        return;
    }
    let (stage, hud_up) = {
        let load = world.resource::<SimulationLoadState>();
        (load.stage, load.hud_spawned)
    };
    if stage == LoadStage::Failed {
        let id = world.resource::<PendingRestore>().id.clone();
        world.remove_resource::<PendingRestore>();
        world.insert_resource(LastRestore(Some(RestoreOutcome::Failed { id })));
        return;
    }
    if !hud_up {
        return;
    }
    if !world.resource::<PendingRestore>().world_applied {
        run_stage(world, Stage::World);
        world.resource_mut::<PendingRestore>().world_applied = true;
        return;
    }
    let spawning = {
        let reg = world.resource::<SessionVehicles>();
        reg.records()
            .iter()
            .any(|r| r.lifecycle == Lifecycle::Spawning)
            || world.contains_resource::<SpawnVehicleRequest>()
    };
    let dt = world.resource::<Time<Real>>().delta_secs();
    let mut timed_out = false;
    {
        let mut pending = world.resource_mut::<PendingRestore>();
        pending.waited_s += dt;
        if spawning {
            pending.quiet_frames = 0;
        } else {
            pending.quiet_frames += 1;
        }
        if pending.waited_s >= SETTLE_TIMEOUT_S {
            timed_out = true;
            pending
                .warnings
                .push("some vehicles did not come back in time".to_owned());
        }
    }
    if !timed_out {
        // Every vehicle exists; give the placement systems a few frames to finish with them.
        if world.resource::<PendingRestore>().quiet_frames < QUIET_FRAMES {
            return;
        }
        if !world.resource::<PendingRestore>().vehicles_applied {
            run_stage(world, Stage::Vehicles);
            world.resource_mut::<PendingRestore>().vehicles_applied = true;
            return;
        }
        // Each vehicle takes its state once its scripts have started.
        crate::session_vehicles::state::apply_pending(world);
        let waiting = world
            .resource::<SessionVehicles>()
            .records()
            .iter()
            .any(|r| r.lifecycle == Lifecycle::Alive && r.saved_state.is_some());
        if waiting {
            return;
        }
    }
    run_stage(world, Stage::Settled);
    let Some(done) = world.remove_resource::<PendingRestore>() else {
        return;
    };
    let failed: Vec<String> = world
        .resource::<SessionVehicles>()
        .records()
        .iter()
        .filter(|r| r.lifecycle == Lifecycle::Failed)
        .map(|r| r.name.clone())
        .collect();
    let mut warnings = done.warnings;
    warnings.extend(
        failed
            .iter()
            .map(|n| format!("vehicle `{n}` could not be rebuilt")),
    );
    tracing::info!(id = %done.id, warnings = warnings.len(), "save restored");
    toast(
        world,
        keys::SAVE_LOADED_TOAST,
        vec![("name", done.name.clone())],
    );
    if !warnings.is_empty() {
        toast(
            world,
            keys::SAVE_LOADED_WARNINGS,
            vec![("count", warnings.len().to_string())],
        );
    }
    world.insert_resource(LastRestore(Some(RestoreOutcome::Done {
        id: done.id,
        warnings,
    })));
}

/// Forgets a restore that was abandoned (the user left the loading screen).
pub fn forget_pending(mut commands: Commands, pending: Option<Res<PendingRestore>>) {
    if pending.is_some() {
        commands.remove_resource::<PendingRestore>();
    }
}
