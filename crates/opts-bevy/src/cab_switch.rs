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

/// A hand-over in progress: the old cab was put in neutral, and the new one takes over once the
/// control cables have carried that to it (the NF6D's second cab deactivates itself when it still
/// sees the first one active).
#[derive(Component, Debug, Clone, Copy)]
pub struct CabHandover {
    /// The cab that takes over.
    pub to: Option<Entity>,
    /// Seconds left before the next step.
    pub left: f32,
    /// `0`: the new cab claims the controls (`kw_s_N`), then the old one learns of it through the
    /// cables and deactivates; `1`: the new cab goes into drive (`automatic_D`).
    pub step: u8,
}

/// Debug: traces the cab flags of the new cab for a while after it took over.
#[derive(Component, Debug, Clone, Copy)]
pub struct CabTrace(pub f32);

/// Logs the flags of the active cab for the first seconds after a hand-over (debug level).
pub fn cab_trace_system(
    time: Res<Time>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut CabTrace, &VehicleScripts, Option<&ActiveCab>)>,
    parts: Query<&VehicleScripts, With<ScriptPart>>,
) {
    for (entity, mut t, head, active) in &mut players {
        t.0 -= time.delta_secs();
        if t.0 <= 0.0 {
            commands.entity(entity).remove::<CabTrace>();
            continue;
        }
        if let Ok(e) = cab_scripts(head, active.copied(), &parts).engine.lock() {
            let g = |n: &str| (e.get(n) * 100.0).round() / 100.0;
            tracing::debug!(
                drive = g("b_kopf_drive"),
                neutral = g("b_kopf_neutral"),
                slw_b = g("slw_b_kopf_inaktiv"),
                slr_a = g("slr_a_kopf_inaktiv"),
                slr_b = g("slr_b_kopf_inaktiv"),
                bat = g("slar_elec_bat"),
                a_drive = g("a_kopf_drive"),
                "cab trace"
            );
        }
    }
}

/// Seconds between putting the old cab in neutral and the new cab taking over.
const HANDOVER_S: f32 = 0.3;
/// Seconds between the new cab's claim and its drive position.
const DRIVE_AFTER_S: f32 = 1.2;

/// The scripts of the active cab: the unit's own engine when a unit is active and has one, else
/// the head's.
#[must_use]
pub fn cab_scripts<'a>(
    head: &'a VehicleScripts,
    active: Option<ActiveCab>,
    parts: &'a Query<&VehicleScripts, With<ScriptPart>>,
) -> &'a VehicleScripts {
    active
        .and_then(|a| a.0)
        .and_then(|e| parts.get(e).ok())
        .unwrap_or(head)
}

/// Whether the active cab is a unit that hangs backwards in the train (a tram's head B): driven
/// from there, the vehicle's forward is the head's backward.
#[must_use]
pub fn cab_is_turned(
    active: Option<&ActiveCab>,
    consist: Option<&crate::consist_scripts::ScriptConsist>,
) -> bool {
    active
        .and_then(|a| a.0)
        .and_then(|e| consist?.units.iter().find(|u| u.part == Some(e)))
        .is_some_and(|u| u.turned)
}

/// The next cab after `current` among `cabs` (wrapping); the first when `current` is not one.
#[must_use]
pub fn next_cab(cabs: &[Option<Entity>], current: Option<Entity>) -> Option<Option<Entity>> {
    if cabs.len() < 2 {
        return None;
    }
    let at = cabs.iter().position(|c| *c == current);
    Some(cabs[at.map_or(0, |i| (i + 1) % cabs.len())])
}

/// Hands a cab over. The cab that gives up presses `kw_s_R` (the NF6D's "deactivate this cab":
/// it marks itself inactive, which the other cab reads through the cables); the cab that takes
/// over presses `kw_s_N` and `automatic_D`, which work only while the other is inactive.
fn hand_over(engine: &mut ScriptEngine, take_over: bool) {
    // The claim and the drive position are pressed one after the other (`cab_handover_system`).
    if !take_over && engine.has_trigger("kw_s_r") {
        engine.trigger("kw_s_r");
    }
}

/// Whether a unit's engine is a cab's: it answers the keys of the driver's seat. (Every unit of
/// an NF6D file lists the driver cameras, so the cameras do not tell the cabs from the joints.)
fn is_cab_engine(scripts: &VehicleScripts) -> bool {
    scripts
        .engine
        .lock()
        .is_ok_and(|e| e.has_trigger("automatic_d") || e.has_trigger("kw_s_n"))
}

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

/// Completes a hand-over: the new cab takes the controls.
pub fn cab_handover_system(
    time: Res<Time>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut CabHandover, &VehicleScripts, &mut RailControls)>,
    parts: Query<&VehicleScripts, With<ScriptPart>>,
) {
    for (entity, mut h, head, mut controls) in &mut players {
        h.left -= time.delta_secs();
        if h.left > 0.0 {
            continue;
        }
        let to = ActiveCab(h.to);
        if h.step == 0 {
            if let Ok(mut new) = cab_scripts(head, Some(to), &parts).engine.lock() {
                if new.has_trigger("kw_s_n") {
                    new.trigger("kw_s_n");
                }
            }
            h.step = 1;
            h.left = DRIVE_AFTER_S;
            continue;
        }
        if let Ok(mut new) = cab_scripts(head, Some(to), &parts).engine.lock() {
            if new.has_trigger("automatic_d") {
                new.trigger("automatic_d");
            }
        }
        controls.lever = 0.0;
        controls.select(1);
        commands
            .entity(entity)
            .insert((to, CabTrace(1.5)))
            .remove::<CabHandover>();
        tracing::info!(cab = ?h.to, "active cab took over");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_cab_wraps_and_a_single_cab_has_no_other() {
        let a = Some(Entity::from_raw_u32(1).unwrap());
        let b = Some(Entity::from_raw_u32(2).unwrap());
        assert_eq!(next_cab(&[None], None), None);
        assert_eq!(next_cab(&[None, a], None), Some(a));
        assert_eq!(next_cab(&[None, a], a), Some(None));
        assert_eq!(next_cab(&[None, a, b], a), Some(b));
        assert_eq!(next_cab(&[None, a, b], b), Some(None));
        // The current cab is not in the list any more: start from the first.
        assert_eq!(next_cab(&[a, b], None), Some(a));
    }
}
