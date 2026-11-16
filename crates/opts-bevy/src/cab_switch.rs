//! Switching the active cab of a coupled rail vehicle: a tram has a head at each end (the NF6D's
//! A and B), a traction of trains has the cabs of every train in it. The player drives from one
//! cab at a time; `rail_switch_cab` (F3) hands the controls to the next cab.
//!
//! The driving is the scripts' own: the cab that is active has the keys, the direction switch and
//! the cockpit camera, the other cabs hear it through the control cables of the vehicle files
//! (`crate::consist_scripts`). A cab is a unit with driver cameras of its own
//! ([`BusVehicle::cab_units`]) and an engine to talk to (the head, or a unit with scripts).

use bevy::prelude::*;
use opts_sim::script::ScriptEngine;

use crate::action_keys::ActionKeys;
use crate::camera::state::{CameraMode, CameraState};
use crate::consist_scripts::ScriptPart;
use crate::rail_controls::RailControls;
use crate::vehicle::BusVehicle;
use crate::vehicle_script::VehicleScripts;
use crate::vehicle_sim::PlayerVehicle;

/// The cab that drives: `None` is the head, `Some(unit)` a unit with a cab of its own.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActiveCab(pub Option<Entity>);
mod handover;
mod queries;
mod trace;

pub use handover::*;
pub use queries::*;
pub use trace::*;

type PlayerCabQueryItem<'a> = (
    Entity,
    &'a BusVehicle,
    &'a VehicleScripts,
    Option<&'a ActiveCab>,
    &'a mut RailControls,
);

/// The cab switch key: moves the controls and the cockpit camera to the next cab.
pub fn cab_switch_system(
    keys: Res<ButtonInput<KeyCode>>,
    action_keys: Option<Res<ActionKeys>>,
    mut state: Option<ResMut<CameraState>>,
    mut commands: Commands,
    mut players: Query<PlayerCabQueryItem, With<PlayerVehicle>>,
    parts: Query<&VehicleScripts, With<ScriptPart>>,
    names: Query<&Name>,
) {
    let action_keys = crate::action_keys::resolve(&action_keys);
    if !action_keys.just_pressed(&keys, "rail_switch_cab") {
        return;
    }
    for (entity, bus, head, active, mut controls) in &mut players {
        // A cab needs an engine to talk to: the head or a unit with scripts.
        let cabs: Vec<Option<Entity>> = bus
            .cab_units()
            .into_iter()
            .filter(|c| c.is_none_or(|e| parts.get(e).is_ok_and(is_cab_engine)))
            .collect();
        tracing::debug!(
            cabs = ?cabs.iter().map(|c| c.and_then(|e| names.get(e).ok()).map(Name::as_str)).collect::<Vec<_>>(),
            "cabs of the consist"
        );
        let current = active.and_then(|a| a.0);
        let Some(next) = next_cab(&cabs, current) else {
            continue;
        };
        if let Ok(mut old) = cab_scripts(head, active.copied(), &parts).engine.lock() {
            hand_over(&mut old, false);
        }
        commands.entity(entity).insert(CabHandover {
            to: next,
            left: HANDOVER_S,
            step: 0,
        });
        controls.lever = 0.0;
        controls.select(1);
        if let Some(state) = state
            .as_deref_mut()
            .filter(|s| s.mode == CameraMode::Cockpit)
        {
            if let Some(i) = bus.first_driver_camera_of(next) {
                state.sub_camera_index = i;
                state.view_yaw = 0.0;
                state.view_pitch = 0.0;
            }
        }
        tracing::info!(cab = ?next, "active cab switched");
    }
}

#[cfg(test)]
mod tests;
