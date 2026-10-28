//! Script-driven drivetrain (`M_Wheel` and brake forces) and vehicle start-up systems.

use super::*;

/// Seconds of throttle with an all-zero `M_Wheel` after which a vehicle falls back to the
/// built-in drivetrain (its const file's torque curve).
const NO_TORQUE_FALLBACK_S: f32 = 4.0;

/// What the scripts hand to the physics: the torque at the driven wheels (`M_Wheel`, N·m) and the
/// total brake force (N). `None` when no script writes `M_Wheel` (the vehicle is not
/// script-driven). Player vehicles write the force of every wheel (`Axle_Brakeforce_<n>_<l|r>`);
/// the total `Brakeforce` belongs to the AI scripts (`{frame_ai}`), so the wheel forces win.
#[must_use]
pub fn script_outputs(engine: &ScriptEngine) -> Option<opts_sim::vehicle::ScriptDrive> {
    if !engine.writes("m_wheel") {
        return None;
    }
    let wheel_names: Vec<String> = (0..8)
        .flat_map(|i| {
            [
                format!("axle_brakeforce_{i}_l"),
                format!("axle_brakeforce_{i}_r"),
            ]
        })
        .collect();
    // A wheel's brake cannot push: the Valiant's script gives a released brake a negative force
    // (cylinder at ambient minus the response pressure), which must not cancel another wheel's.
    let brake = if wheel_names.iter().any(|n| engine.writes(n)) {
        wheel_names.iter().map(|n| engine.get(n).max(0.0)).sum()
    } else {
        engine.get("brakeforce")
    };
    Some(opts_sim::vehicle::ScriptDrive {
        wheel_torque_nm: engine.get("m_wheel") * opts_sim::vehicle::M_WHEEL_UNIT,
        brake_force_n: brake.max(0.0) * opts_sim::vehicle::BRAKE_FORCE_UNIT,
    })
}

/// What [`script_drive_system`] remembers of one vehicle's scripts.
#[derive(Debug, Default)]
pub struct DriveWatch {
    idle_s: f32,
    fallback: bool,
    warned: bool,
}

/// What [`script_drive_system`] reads and writes of a vehicle.
type DriveBody = (
    Entity,
    &'static mut VehicleController,
    &'static VehicleScripts,
    Option<&'static crate::consist_scripts::ScriptConsist>,
    Has<crate::rail_controls::RailControls>,
);

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
        // The pedals are the player's: the feed hands them to the scripts every frame, also where
        // a script writes `Throttle` or `Brake` for a corner case (copying the script's value back
        // into the pedal made a zero stick: the HH109 1992 never got its throttle).
        if !enabled.0 {
            controller.sim.script_drive = None;
            continue;
        }
        let Ok(engine) = scripts.engine.lock() else {
            continue;
        };
        // The scripts' spring factors, wheel by wheel.
        for i in 0..controller.sim.axles.len().min(8) {
            for (side, tag) in ["l", "r"].into_iter().enumerate() {
                let name = format!("axle_springfactor_{i}_{tag}");
                if engine.writes(&name) {
                    controller
                        .sim
                        .set_spring_factor(i * 2 + side, engine.get(&name));
                }
            }
        }
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
        // A rail consist: every unit with scripts of its own drives and brakes its own wheels (the
        // motor cars of a traction pull with their own scripts); a unit that hangs backwards
        // writes its torque in its own frame.
        if rail {
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
        // Three ways to drive: a script that hands torque to the wheels; one that never does
        // while the driver holds the pedal (const file's torque curve, one INFO); none at all
        // (built-in constants, one WARN, above).
        if drive.wheel_torque_nm.abs() > 1.0 || engine.get("bremse_feststell") >= 0.5 {
            w.idle_s = 0.0;
            w.fallback = false;
        } else if controller.inputs().throttle() > 0.2
            && engine.get("engine_on") > 0.5
            // A parking brake the script holds (the Urbino's spring brake until its air is up)
            // is why it hands no torque: the built-in drivetrain must not drive through it.
            && engine.get("bremse_feststell") < 0.5
        {
            w.idle_s += time.delta_secs();
            if w.idle_s > NO_TORQUE_FALLBACK_S && !w.fallback {
                w.fallback = true;
                tracing::info!(
                    "the scripts of a vehicle hand no torque to the wheels under throttle: the const file's torque curve drives it"
                );
            }
        }
        if w.fallback {
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

/// Start-up of the scripts' vehicle and gear selector (see [`ScriptStartup`]): the player's
/// [`VehicleSetup`](crate::vehicle_setup::VehicleSetup) says how far the vehicle is brought.
pub fn script_startup_system(
    time: Res<Time>,
    enabled: Res<ScriptDriveEnabled>,
    setup: Option<Res<crate::vehicle_setup::VehicleSetup>>,
    mut query: Query<(
        &mut VehicleController,
        &VehicleScripts,
        &mut ScriptStartup,
        Has<crate::rail_controls::RailControls>,
    )>,
) {
    if !enabled.0 {
        return;
    }
    let setup = setup.map_or_else(Default::default, |s| *s);
    for (mut controller, scripts, mut st, rail) in &mut query {
        st.elapsed += time.delta_secs();
        let Ok(mut engine) = scripts.engine.lock() else {
            continue;
        };
        let now = st.elapsed;
        let settled = st.run.advance(&mut engine, &setup, now);
        if settled != st.settled {
            st.settled = settled;
            tracing::debug!(
                busbar = engine.get("elec_busbar_main"),
                gear_select = engine.get("antrieb_getr_gangwahl"),
                engine_n = engine.get("engine_n"),
                engine_on = engine.get("engine_on"),
                degree = setup.degree,
                "script start-up settled"
            );
        }
        // Ready to drive: the parking brake comes off once, when the start-up has settled. A rail
        // vehicle has no key for it and starts free at once.
        let release = if rail {
            true
        } else {
            settled
                && setup.degree >= crate::vehicle_setup::READY
                && (!crate::vehicle_setup::has_engine(&engine)
                    || crate::vehicle_setup::ready(&engine))
        };
        if release && controller.sim.inputs.is_parking_brake_engaged() && !st.released {
            st.released = true;
            controller.sim.inputs.set_parking_brake(false);
        }
        let gear = controller.sim.inputs.gear();
        // The gear selector follows the player once the engine is up (before: neutral/park).
        let gear = if st.settled {
            gear
        } else {
            opts_sim::GearPosition::Neutral
        };
        if st.gear != Some(gear) {
            st.gear = Some(gear);
            let name = match gear {
                opts_sim::GearPosition::Drive | opts_sim::GearPosition::Forward(_) => "automatic_d",
                opts_sim::GearPosition::Reverse => "automatic_r",
                opts_sim::GearPosition::Neutral => "automatic_n",
            };
            // The automatic only leaves neutral with the brake pressed (an interlock of the
            // scripts); the player's selection stands for that press.
            let brake = engine.get("brake");
            engine.set("brake", 1.0);
            engine.trigger(name);
            engine.set("brake", brake);
        }
        // The parking brake of the scripts follows the player's, once per change. A vehicle whose
        // lever is `cockpit_parking_brake` (the Urbino's: 1 is *released*, and `bremse_feststell`
        // also shows applied while its spring brake lacks air, which is no lever position) gets
        // its idempotent `parking_brake_set` / `parking_brake_release`; the others are toggled by
        // what `bremse_feststell` shows (their `set` and `release` lean on the condition staying
        // on the stack after `{if}`, which the VM pops).
        let parking = controller.sim.inputs.is_parking_brake_engaged();
        if st.parking != Some(parking) {
            if engine.writes("cockpit_parking_brake") {
                engine.trigger(if parking {
                    "parking_brake_set"
                } else {
                    "parking_brake_release"
                });
            } else if parking != (engine.get("bremse_feststell") > 0.5) {
                engine.trigger("parking_brake_toggle");
            }
        }
        st.parking = Some(parking);
    }
}
