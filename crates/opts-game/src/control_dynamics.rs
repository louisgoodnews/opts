//! Control protocol of the vehicle dynamics (M-0073): `vehicle.dynamics` answers what the
//! player's vehicle states about itself (mass, centre of gravity, inertia, rolling resistance,
//! axles with spring, damper and force) with the source of each value (the vehicle file, an
//! estimate from it, or the generic default), and what the simulation holds live: the speed,
//! the mass the dynamics move and the load on every axle.

use bevy::prelude::*;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::{PlayerVehicle, VehicleController};
use opts_cli::protocol::{code, RpcError};
use opts_sim::vehicle::spec::Sourced;
use opts_sim::VehicleSimulation;
use serde_json::{json, Value};

/// Registers `vehicle.dynamics`.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("vehicle.dynamics", dynamics);
}

fn sourced(s: Sourced) -> Value {
    json!({ "value": s.value, "source": s.source.name() })
}

/// The answer for one simulation.
#[must_use]
pub fn describe(sim: &VehicleSimulation) -> Value {
    let positions: Vec<f32> = sim.axles.iter().map(|a| a.config.longitudinal_m).collect();
    let static_loads = sim
        .suspension
        .compute_static_loads(&positions, sim.chassis.total_mass_kg);
    let axles: Vec<Value> = sim
        .axles
        .iter()
        .enumerate()
        .map(|(i, a)| {
            json!({
                "index": i,
                "longitudinal_m": a.config.longitudinal_m,
                "track_width_m": a.config.track_width_m,
                "wheel_diameter_m": a.config.wheel_diameter_m,
                "spring_n_per_m": a.config.spring_rate_n_m,
                "damping_n_s_per_m": a.config.damping_n_s_m,
                "max_force_n": a.config.max_force_n,
                "driven": a.config.is_driven,
                "steered": a.config.is_steered,
                "static_load_n": static_loads.get(i).copied(),
                "load_n": [a.left_wheel.vertical_load_n, a.right_wheel.vertical_load_n],
                "deflection_m": a.suspension_deflection_m,
                "deflection_speed_m_s": a.suspension_velocity_m_s,
            })
        })
        .collect();
    json!({
        "spec": {
            "mass_kg": sourced(sim.spec.mass_kg),
            "cg_height_m": sourced(sim.spec.cg_height_m),
            "yaw_inertia_kg_m2": sourced(sim.spec.yaw_inertia_kg_m2),
            "rolling_coefficient": sourced(sim.spec.rolling_coefficient),
        },
        "axles": axles,
        "state": {
            "speed_m_s": sim.chassis.forward_speed_m_s(),
            "head_mass_kg": sim.head_mass_kg(),
            "consist_mass_kg": sim.consist_mass_kg(),
            "units": 1 + sim.consist.len(),
            "wheelbase_m": sim.wheelbase_m,
        },
    })
}

fn dynamics(world: &mut World, _: &Value) -> Outcome {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    match q.iter(world).next() {
        Some(c) => Outcome::ok(describe(&c.sim)),
        None => Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (start a drive first)",
        )),
    }
}
