//! The scripts of a coupled vehicle's units and the control cables between them
//! (`docs/src/roadmap/phase-4/M-0098.md`).
//!
//! The head runs its scripts in [`crate::vehicle_script::vehicle_script_system`]; every further
//! unit that has scripts of its own (a tram's second and third car, its bogies) carries an engine
//! on its own entity ([`ScriptPart`]) and is stepped right after the head, front to back. Units
//! with `[scriptshare]` have none: they read the engine of the unit that owns their store.
//! After all of them ran, the cables of the vehicle files copy values between the engines
//! (`opts_sim::script::consist`).

use bevy::prelude::*;
use opts_sim::script::consist::{bridge, Cable, Unit};
use opts_sim::script::inputs::{feed, HostInputs, DOORS};
use opts_sim::script::ScriptEngine;

use crate::vehicle_script::{PassengerFeed, VehicleScripts};

/// A unit with a script engine of its own, behind the head of its consist.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ScriptPart;

/// One unit of a consist as the cable bridge sees it.
#[derive(Debug, Clone)]
pub struct ConsistUnit {
    /// The unit's entity when it has an engine of its own; `None` for the head (whose engine is
    /// the one the system holds) and for units that share another's scripts.
    pub part: Option<Entity>,
    /// Whether the head's engine is this unit's.
    pub head: bool,
    /// The cables its vehicle file declares.
    pub cables: Vec<Cable>,
    /// The vehicle of the formation it belongs to (0: the head's own chain).
    pub vehicle: usize,
}

/// The units of a consist, head first, on the head's entity.
#[derive(Component, Debug, Clone, Default)]
pub struct ScriptConsist {
    /// Front to back.
    pub units: Vec<ConsistUnit>,
}

/// Steps the engines of the consist's parts with the head's inputs, then bridges the cables.
pub fn run_parts(
    consist: &ScriptConsist,
    parts: &mut Query<(&mut VehicleScripts, Option<&PassengerFeed>), With<ScriptPart>>,
    head: &mut ScriptEngine,
    inputs: &HostInputs,
    dt: f32,
) {
    for unit in &consist.units {
        let Some((mut scripts, pax)) = unit.part.and_then(|e| parts.get_mut(e).ok()) else {
            continue;
        };
        // The part hears about its own doors, not the head's.
        let mut inputs = inputs.clone();
        inputs.humans_count = 0;
        inputs.pax_entry_req = pax.map_or([false; DOORS], |p| p.entry_req);
        inputs.pax_exit_req = pax.map_or([false; DOORS], |p| p.exit_req);
        let first = !scripts.initialised;
        scripts.initialised = true;
        let VehicleScripts {
            engine,
            plan,
            raised,
            ..
        } = &mut *scripts;
        let Ok(engine) = engine.get_mut() else {
            continue;
        };
        feed(engine, plan, &inputs);
        if first {
            engine.run_init();
        }
        engine.run_frame(dt, false);
        for d in engine.drain_diagnostics() {
            tracing::warn!(file = %d.file, line = d.line, "{}", d.message);
        }
        for e in engine.drain_events() {
            if raised.len() < 256 {
                raised.push(e.name);
            }
        }
    }
    let mut guards: Vec<_> = consist
        .units
        .iter()
        .map(|u| {
            u.part
                .and_then(|e| parts.get(e).ok())
                .and_then(|(s, _)| s.engine.lock().ok())
        })
        .collect();
    let mut head = Some(head);
    let mut units: Vec<Unit> = Vec::with_capacity(guards.len());
    for (u, guard) in consist.units.iter().zip(guards.iter_mut()) {
        let engine: Option<&mut ScriptEngine> = if u.head {
            head.take()
        } else {
            guard.as_deref_mut()
        };
        units.push(Unit {
            engine,
            cables: &u.cables,
            vehicle: u.vehicle,
        });
    }
    bridge(&mut units);
}
