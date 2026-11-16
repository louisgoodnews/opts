//! Reading the configurations for the replies (`config.list`): the repaints of a vehicle, the
//! catalogue, and the JSON of a configuration with its checks.

use std::cell::RefCell;
use std::collections::HashMap;

use bevy::prelude::*;
use opts_bevy::control::state::{Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Map, Value};

use crate::bus_config::{BusConfiguration, Checked};
use crate::new_game_screen::livery::LiveryState;
use crate::new_game_screen::repaints::{vehicle_repaints, VehicleRepaints};
use crate::new_game_screen::types::{GameCatalogue, VehicleItem};

use super::open;

/// Repaints of a vehicle: the wizard's own discovery when it is for this vehicle, else a fresh
/// one (reads the vehicle's files).
pub(super) struct Repaints {
    known: Option<(String, VehicleRepaints)>,
    cache: RefCell<HashMap<String, VehicleRepaints>>,
}

impl Repaints {
    pub(super) fn new(world: &World) -> Self {
        let known = world.get_resource::<LiveryState>().and_then(|l| {
            l.repaints
                .clone()
                .map(|r| (l.vehicle.clone(), r))
                .filter(|(v, _)| !v.is_empty())
        });
        Self {
            known,
            cache: RefCell::default(),
        }
    }

    pub(super) fn of(&self, v: &VehicleItem) -> VehicleRepaints {
        if let Some((id, r)) = &self.known {
            if *id == v.id {
                return r.clone();
            }
        }
        self.cache
            .borrow_mut()
            .entry(v.id.clone())
            .or_insert_with(|| vehicle_repaints(v))
            .clone()
    }
}

pub(super) fn catalogue(world: &World) -> GameCatalogue {
    world
        .get_resource::<GameCatalogue>()
        .cloned()
        .unwrap_or_default()
}

pub(super) fn describe(c: &BusConfiguration, cat: &GameCatalogue, checked: Option<&Checked>) -> Value {
    let setvars: Map<String, Value> = c
        .setvars
        .iter()
        .map(|(n, v)| (n.clone(), json!(v)))
        .collect();
    let mut out = json!({
        "id": c.id,
        "name": c.name,
        "vehicle": c.vehicle,
        "vehicle_name": cat.vehicles.iter().find(|v| v.id == c.vehicle).map(|v| v.full_name.clone()),
        "repaint": c.repaint,
        "setvars": setvars,
        "notes": c.notes,
        "favourite": c.favourite,
        "created": c.created,
        "last_used": c.last_used,
    });
    if let Some(ch) = checked {
        out["usable"] = json!(ch.usable());
        out["problems"] = Value::Array(
            ch.problems
                .iter()
                .map(|p| json!({ "code": p.code(), "message": p.to_string() }))
                .collect(),
        );
    }
    out
}

pub(super) fn not_found(key: &str) -> RpcError {
    RpcError::new(code::NOT_FOUND, format!("no configuration `{key}`"))
}

pub(super) fn list(world: &mut World, _: &Value) -> Outcome {
    let (_, store) = match open(world) {
        Ok(s) => s,
        Err(e) => return Outcome::err(e),
    };
    let cat = catalogue(world);
    let reps = Repaints::new(world);
    let items: Vec<Value> = store
        .configurations
        .iter()
        .map(|c| describe(c, &cat, Some(&c.check(&cat, |v| reps.of(v)))))
        .collect();
    Outcome::ok(json!({ "count": items.len(), "configurations": items }))
}
