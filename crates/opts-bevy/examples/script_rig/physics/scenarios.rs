//! The closed-loop scenarios of the physics report: acceleration, braking, parking grade and the throttle sweep.

use super::super::phases::Loop;

/// Builds a fresh, not yet started loop of the vehicle.
pub(super) type Fresh<'a> = &'a dyn Fn() -> Loop;

/// Trace line of the acceleration scenario (`RIG_TRACE_ACCEL`).
fn trace_accel(a: &Loop, t0: f32) {
    let d = a.controller.sim.script_drive.unwrap_or_default();
    eprintln!(
        "accel t={:.0} v={:.1} km/h torque={:.0} brake={:.0} n={:.0} gear={} halte={:.0}",
        a.t - t0,
        a.speed() * 3.6,
        d.wheel_torque_nm,
        d.brake_force_n,
        a.engine.get("engine_n"),
        a.engine.get("antrieb_getr_aktugang"),
        a.engine.get("bremse_halte"),
    );
    if let Ok(list) = std::env::var("RIG_VARS") {
        let extra: Vec<String> = list
            .split(',')
            .map(|n| format!("{n}={:.3}", a.engine.get(&n.to_lowercase())))
            .collect();
        eprintln!("  vars {}", extra.join(" "));
    }
}

/// 1. Standstill to 50 km/h at full throttle: the loop, the time to 50 km/h and the speed
///    after 40 s (km/h).
pub(super) fn accel_scenario(fresh: Fresh<'_>, dt: f32, hz: f32) -> (Loop, Option<f32>, f32) {
    let mut a = fresh();
    a.ready();
    a.controller.inputs_mut().set_throttle(1.0);
    let t0 = a.t;
    let mut to_50 = None;
    while a.t - t0 < 40.0 {
        a.tick();
        if std::env::var_os("RIG_TRACE_ACCEL").is_some()
            && (((a.t - t0) / dt) as usize).is_multiple_of(hz as usize)
        {
            trace_accel(&a, t0);
        }
        if to_50.is_none() && a.speed() * 3.6 >= 50.0 {
            to_50 = Some(a.t - t0);
        }
    }
    let speed_after_40 = a.speed() * 3.6;
    (a, to_50, speed_after_40)
}

/// Trace line of the braking scenario (`RIG_TRACE_BRAKE`).
fn trace_brake(b: &Loop, t1: f32) {
    eprintln!(
        "brake t={:.2} v={:.2} force={:.0} tank={:.0} luft={:.3} zielBBA={:.0} cyl_HA_R={:.0} pedal_in={:.2}",
        b.t - t1,
        b.speed(),
        b.controller
            .sim
            .script_drive
            .map_or(0.0, |d| d.brake_force_n),
        b.engine.get("bremse_p_tank01"),
        b.engine.get("bremse_luft"),
        b.engine.get("bremse_zieldruck_bba"),
        b.engine.get("bremse_p_brzyl_ha_r"),
        b.engine.get("brake"),
    );
}

/// 2. Stop from 50 km/h at full brake: the loop, the stopping distance (m) and time (s).
pub(super) fn brake_scenario(fresh: Fresh<'_>, dt: f32) -> (Loop, f32, f32) {
    let mut b = fresh();
    b.ready();
    b.controller.sim.chassis.local_velocity = bevy::math::Vec2::new(0.0, 50.0 / 3.6);
    b.controller.inputs_mut().set_service_brake(1.0);
    let start = b.controller.sim.chassis.position.y;
    let t1 = b.t;
    while b.t - t1 < 20.0 && b.speed() > 0.05 {
        b.tick();
        if std::env::var_os("RIG_TRACE_BRAKE").is_some()
            && (((b.t - t1) / dt) as usize).is_multiple_of(12)
        {
            trace_brake(&b, t1);
        }
    }
    let stop_m = b.controller.sim.chassis.position.y - start;
    let stop_s = b.t - t1;
    (b, stop_m, stop_s)
}

/// 3. Parking brake on a 10 % grade, facing uphill: the loop and the largest drift (m).
pub(super) fn grade_scenario(fresh: Fresh<'_>, dt: f32) -> (Loop, f32) {
    let mut c = fresh();
    c.ready();
    c.controller.inputs_mut().set_parking_brake(true);
    // The driver pulls the lever: the scripts' spring brake follows (as in the game's start-up
    // system, which asks for the toggle when the two differ).
    if c.engine.get("bremse_feststell") < 0.5 {
        c.engine.trigger("parking_brake_toggle");
    }
    c.controller.sim.head_body = Some(opts_sim::vehicle::ground::UnitBody {
        elevation: 0.0,
        pitch: 0.10f32.atan(),
        roll: 0.0,
    });
    let y0 = c.controller.sim.chassis.position.y;
    let mut drift = 0.0f32;
    for _ in 0..(25.0 / dt) as usize {
        c.tick();
        drift = drift.max((c.controller.sim.chassis.position.y - y0).abs());
    }
    (c, drift)
}

/// 4. A throttle sweep: the speed (km/h) after 8 s at each throttle.
pub(super) fn sweep_scenario(fresh: Fresh<'_>) -> Vec<f32> {
    [0.2f32, 0.4, 0.6, 0.8, 1.0]
        .iter()
        .map(|&th| {
            let mut s = fresh();
            s.ready();
            s.controller.inputs_mut().set_throttle(th);
            s.run(8.0);
            s.speed() * 3.6
        })
        .collect()
}
