//! Start-up of the scripts' vehicle and gear selector, and the parking brake that follows the player.

use super::*;

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
