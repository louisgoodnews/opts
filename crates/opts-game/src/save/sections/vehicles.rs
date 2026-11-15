//! The `session_vehicles` and `active_vehicle` save sections: the vehicles of the session and which one the driver is in.

use std::path::Path;

use bevy::prelude::*;
use serde_json::{json, Value};

use super::super::registry::{CheckEnv, Issue, IssueKind};
use super::super::fingerprint;
use super::resolve_bus_file;
use crate::in_game::omsi_world::WorldLoadJob;
use crate::session_vehicles::{ops, EntrySel, SessionVehicles, SAVE_SECTION_VERSION};

// ---- session vehicles ------------------------------------------------------------------------

pub(super) fn root_of(world: &World) -> Option<std::path::PathBuf> {
    world
        .get_resource::<WorldLoadJob>()
        .map(|j| j.omsi_root().to_path_buf())
        .or_else(crate::omsi_root::known_omsi_root)
}

pub(super) fn vehicles_capture(world: &mut World) -> Result<Option<Value>, String> {
    let mut reg = world.resource::<SessionVehicles>().clone();
    let root = root_of(world);
    let ids: Vec<_> = reg
        .records()
        .iter()
        .filter(|r| r.is_alive())
        .map(|r| (r.id, r.entity))
        .collect();
    for (id, entity) in ids {
        let state = entity.and_then(|e| crate::session_vehicles::state::capture(world, e));
        if let Some(r) = reg.get_mut(id) {
            r.saved_state = state;
        }
    }
    let rels: Vec<_> = reg
        .records()
        .iter()
        .map(|r| {
            (
                r.id,
                root.as_deref().and_then(|root| {
                    Path::new(&r.bus_file)
                        .strip_prefix(root)
                        .ok()
                        .map(|p| p.to_string_lossy().replace('\\', "/"))
                }),
            )
        })
        .collect();
    for (id, rel) in rels {
        if let Some(r) = reg.get_mut(id) {
            r.bus_rel = rel;
        }
    }
    Ok(Some(reg.to_section()))
}

pub(super) fn vehicles_migrate(from: u32, mut data: Value) -> Result<Value, String> {
    match from {
        // Version 2 added `saved_state` and `bus_rel` to the records; both default when absent.
        1 => {
            data["version"] = json!(SAVE_SECTION_VERSION);
            Ok(data)
        }
        other => Err(format!("no migration from version {other}")),
    }
}

pub(super) fn vehicles_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let mut reg = SessionVehicles::from_section(data)?;
    let root = root_of(world);
    let ids: Vec<_> = reg.records().iter().map(|r| r.id).collect();
    for id in ids {
        let Some(r) = reg.get_mut(id) else { continue };
        if let Some(path) = resolve_bus_file(&r.bus_file, r.bus_rel.as_deref(), root.as_deref()) {
            r.bus_file = path.display().to_string();
        }
        // The entry point it really stood at is the most exact request there is.
        if let Some(entry) = r.spawn_entry_id {
            r.entry = EntrySel::Id(entry);
        }
    }
    // The active vehicle is restored last (`active_vehicle`): building a vehicle activates it.
    reg.set_active(None);
    *world.resource_mut::<SessionVehicles>() = reg;
    Ok(())
}

pub(super) fn vehicles_check(data: &Value, env: &CheckEnv) -> Vec<Issue> {
    let records = data
        .get("vehicles")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut issues = Vec::new();
    let mut missing = Vec::new();
    for r in &records {
        let text = |key| r.get(key).and_then(Value::as_str);
        let saved = text("bus_file").unwrap_or_default();
        let name = text("name").unwrap_or(saved).to_owned();
        match resolve_bus_file(saved, text("bus_rel"), env.root) {
            None => missing.push(name),
            Some(path) => {
                let key = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                if let (Some(old), Some(now)) = (
                    env.fingerprint.vehicles.get(&key),
                    fingerprint::vehicle(&path),
                ) {
                    if *old != now {
                        issues.push(Issue {
                            kind: IssueKind::Changed,
                            subject: name,
                            blocking: false,
                        });
                    }
                }
            }
        }
    }
    // Missing vehicles block the load only when none of the saved vehicles can be rebuilt.
    let all_missing = !records.is_empty() && missing.len() == records.len();
    issues.extend(missing.into_iter().map(|subject| Issue {
        kind: IssueKind::MissingVehicle,
        subject,
        blocking: all_missing,
    }));
    issues
}

// ---- active vehicle --------------------------------------------------------------------------

pub(super) fn active_capture(world: &mut World) -> Result<Option<Value>, String> {
    let reg = world.resource::<SessionVehicles>();
    Ok(Some(json!({ "id": reg.active_id() })))
}

pub(super) fn active_apply(world: &mut World, data: &Value) -> Result<(), String> {
    let Some(id) = data
        .get("id")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
    else {
        return Ok(());
    };
    let alive = world
        .resource::<SessionVehicles>()
        .get(id)
        .is_some_and(|r| r.is_alive());
    if alive {
        ops::switch_to(world, id).map_err(|e| e.to_string())?;
    }
    Ok(())
}
