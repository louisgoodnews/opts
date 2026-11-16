//! Feeding the scripts' torque and brake force to the physics each step, and the fallback to the
//! built-in drivetrain when the scripts hand no torque.

use super::*;

/// What [`script_drive_system`] reads and writes of a vehicle.
type DriveBody = (
    Entity,
    &'static mut VehicleController,
    &'static VehicleScripts,
    Option<&'static crate::consist_scripts::ScriptConsist>,
    Has<crate::rail_controls::RailControls>,
);

fn apply_spring_factors(sim: &mut opts_sim::VehicleSimulation, engine: &ScriptEngine) {
    for i in 0..sim.axles.len().min(8) {
        for (side, tag) in ["l", "r"].into_iter().enumerate() {
            let name = format!("axle_springfactor_{i}_{tag}");
            if engine.writes(&name) {
                sim.set_spring_factor(i * 2 + side, engine.get(&name));
            }
        }
    }
}

fn accumulate_rail_drive(
    drive: &mut opts_sim::vehicle::ScriptDrive,
    consist: Option<&crate::consist_scripts::ScriptConsist>,
    parts: &Query<&VehicleScripts, With<crate::consist_scripts::ScriptPart>>,
) {
    for u in consist.iter().flat_map(|c| c.units.iter()) {
        let Some(part) = u.part.filter(|_| !u.head) else {
            continue;
        };
        let Ok(scripts) = parts.get(part) else {
            continue;
        };
        let Ok(unit_engine) = scripts.engine.lock() else {
            continue;
        };
        if let Some(d) = script_outputs(&unit_engine) {
            let sign = if u.turned { -1.0 } else { 1.0 };
            drive.wheel_torque_nm += sign * d.wheel_torque_nm;
            drive.brake_force_n += d.brake_force_n;
        }
    }
}

fn update_drive_watch(
    w: &mut DriveWatch,
    drive: &opts_sim::vehicle::ScriptDrive,
    engine: &ScriptEngine,
    throttle: f32,
    delta_s: f32,
) -> bool {
    if drive.wheel_torque_nm.abs() > 1.0 || engine.get("bremse_feststell") >= 0.5 {
        w.idle_s = 0.0;
        w.fallback = false;
    } else if throttle > 0.2
        && engine.get("engine_on") > 0.5
        && engine.get("bremse_feststell") < 0.5
    {
        w.idle_s += delta_s;
        if w.idle_s > NO_TORQUE_FALLBACK_S && !w.fallback {
            w.fallback = true;
            tracing::info!(
                "the scripts of a vehicle hand no torque to the wheels under throttle: the const file's torque curve drives it"
            );
        }
    }
    w.fallback
}

/// Hands the scripts' torque and brake force to the physics for the next step (only when
/// [`ScriptDriveEnabled`] and the scripts write `M_Wheel`).
pub fn script_drive_system(
    time: Res<Time>,
    enabled: Res<ScriptDriveEnabled>,
    mut ticks: Local<u32>,
    mut watch: Local<std::collections::HashMap<Entity, DriveWatch>>,
    mut query: Query<DriveBody>,
    parts: Query<&VehicleScripts, With<crate::consist_scripts::ScriptPart>>,
) {
    for (entity, mut controller, scripts, consist, rail) in &mut query {
        if !enabled.0 {
            controller.sim.script_drive = None;
            continue;
        }
        let Ok(engine) = scripts.engine.lock() else {
            continue;
        };
        apply_spring_factors(&mut controller.sim, &engine);

        let w = watch.entry(entity).or_default();
        let Some(mut drive) = script_outputs(&engine) else {
            if !w.warned {
                w.warned = true;
                tracing::warn!(
                    "the scripts of a vehicle write no `M_Wheel`: the built-in drivetrain with its constants drives it"
                );
            }
            controller.sim.script_drive = None;
            continue;
        };

        if rail {
            accumulate_rail_drive(&mut drive, consist, &parts);
        }

        if update_drive_watch(
            w,
            &drive,
            &engine,
            controller.inputs().throttle(),
            time.delta_secs(),
        ) {
            controller.sim.script_drive = None;
            continue;
        }

        *ticks = ticks.wrapping_add(1);
        if (*ticks).is_multiple_of(64) {
            tracing::debug!(
                m_wheel = drive.wheel_torque_nm,
                brake = drive.brake_force_n,
                engine_n = engine.get("engine_n"),
                throttle = engine.get("throttle"),
                gear_pre = engine.get("antrieb_getr_gangvorwahl"),
                gear_sel = engine.get("antrieb_getr_gangwahl"),
                gear_act = engine.get("antrieb_getr_aktugang"),
                parking = engine.get("bremse_feststell"),
                tank = engine.get("bremse_p_tank01"),
                "script drive"
            );
        }
        controller.sim.script_drive = Some(drive);
    }
}
