//! Applying a restore in stages as the session comes up: the world sections, the vehicles, and the
//! settled sections, then the outcome is recorded and toasted.

use bevy::prelude::*;
use opts_gui::components::toast::ToastSeverity;

use super::super::registry::Stage;
use super::types::{LastRestore, PendingRestore, RestoreOutcome, QUIET_FRAMES, SETTLE_TIMEOUT_S};
use crate::in_game::omsi_world::SpawnVehicleRequest;
use crate::in_game::types::{LoadStage, SimulationLoadState};
use crate::session_vehicles::{Lifecycle, SessionVehicles};
use crate::shell::keys;
use crate::shell::notify::NotifyExt;

pub(super) fn run_stage(world: &mut World, stage: Stage) {
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

pub(super) fn toast(world: &mut World, key: &'static str, args: Vec<(&'static str, String)>) {
    world.commands().notify(ToastSeverity::Info, key, args);
    world.flush();
}

/// Waits for the restored vehicles: all spawned, quiet for a few frames, their state applied.
/// Returns true while it still has to wait; a timeout counts as done and adds a warning.
pub(super) fn settle_vehicles(world: &mut World) -> bool {
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
            return true;
        }
        if !world.resource::<PendingRestore>().vehicles_applied {
            run_stage(world, Stage::Vehicles);
            world.resource_mut::<PendingRestore>().vehicles_applied = true;
            return true;
        }
        // Each vehicle takes its state once its scripts have started.
        crate::session_vehicles::state::apply_pending(world);
        let waiting = world
            .resource::<SessionVehicles>()
            .records()
            .iter()
            .any(|r| r.lifecycle == Lifecycle::Alive && r.saved_state.is_some());
        if waiting {
            return true;
        }
    }
    false
}

/// The restore is complete: reports the failed vehicles, toasts and records the outcome.
pub(super) fn finish_restore(world: &mut World) {
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
    if settle_vehicles(world) {
        return;
    }
    finish_restore(world);
}

/// Forgets a restore that was abandoned (the user left the loading screen).
pub fn forget_pending(mut commands: Commands, pending: Option<Res<PendingRestore>>) {
    if pending.is_some() {
        commands.remove_resource::<PendingRestore>();
    }
}
