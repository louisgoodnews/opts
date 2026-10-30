//! The physics report and its golden values.

use super::phases::Loop;
use super::*;

/// Variables of a golden run, sampled at [`GOLDEN_TIMES`] (seconds).
pub(super) const GOLDEN_VARS: &[&str] = &[
    "elec_busbar_main",
    "engine_on",
    "engine_n",
    "m_wheel",
    "antrieb_getr_aktugang",
    "antrieb_getr_gangwahl",
    "bremse_p_tank01",
    "bremse_feststell",
    "axle_brakeforce_0_l",
    "axle_brakeforce_1_l",
    "brakeforce",
    "n_wheel",
];

pub(super) const GOLDEN_TIMES: &[f32] = &[3.0, 6.0, 9.0, 11.0];

/// Builds a fresh, not yet started loop of the vehicle.
type Fresh<'a> = &'a dyn Fn() -> Loop;

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

/// 1. Standstill to 50 km/h at full throttle: the loop, the time to 50 km/h and the speed after
/// 40 s (km/h).
fn accel_scenario(fresh: Fresh<'_>, dt: f32, hz: f32) -> (Loop, Option<f32>, f32) {
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
fn brake_scenario(fresh: Fresh<'_>, dt: f32) -> (Loop, f32, f32) {
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
fn grade_scenario(fresh: Fresh<'_>, dt: f32) -> (Loop, f32) {
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
fn sweep_scenario(fresh: Fresh<'_>) -> Vec<f32> {
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

/// The closed-loop scenarios of one vehicle (`--physics`).
pub(super) fn physics_report(
    bus: &BusManifest,
    trail: Option<&BusManifest>,
    load: &dyn Fn() -> opts_sim::script::ScriptEngine,
    hz: f32,
) -> serde_json::Value {
    let dt = 1.0 / hz;
    let fresh = || Loop::new(bus, trail, load(), dt);
    let (a, to_50, speed_after_40) = accel_scenario(&fresh, dt, hz);
    let (b, stop_m, stop_s) = brake_scenario(&fresh, dt);
    let (c, drift) = grade_scenario(&fresh, dt);
    let sweep = sweep_scenario(&fresh);
    let monotone = sweep.windows(2).all(|w| w[1] >= w[0] - 0.05);
    serde_json::json!({
        "hz": hz,
        "mass_kg": bus.mass_tonnes.map(|t| t * 1000.0),
        "seconds_to_50_kmh": to_50,
        "speed_after_40_s_kmh": speed_after_40,
        "stop_from_50_kmh_m": stop_m,
        "stop_from_50_kmh_s": stop_s,
        "stopped": b.speed() <= 0.05,
        "parking_brake_10_percent_drift_m": drift,
        "parking_brake_10_percent_final_speed": c.speed(),
        "throttle_sweep_kmh_after_8_s": sweep,
        "throttle_sweep_monotone": monotone,
        "engine_on": a.engine.get("engine_on"),
    })
}

/// Compares two physics reports; numbers may differ by 5 % (or 0.2 absolute: stopping and
/// hold distances are small), `hz` is not compared (the golden is for the step size it names).
pub(super) fn physics_diff(
    at: &str,
    want: &serde_json::Value,
    got: &serde_json::Value,
    out: &mut Vec<String>,
) {
    use serde_json::Value::{Array, Number, Object};
    match (want, got) {
        (Object(a), Object(b)) => {
            for (k, v) in a {
                if k == "hz" {
                    continue;
                }
                physics_diff(
                    &format!("{at}.{k}"),
                    v,
                    b.get(k).unwrap_or(&serde_json::Value::Null),
                    out,
                );
            }
        }
        (Array(a), Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                physics_diff(&format!("{at}[{i}]"), x, y, out);
            }
        }
        (Number(a), Number(b)) => {
            let (a, b) = (a.as_f64().unwrap_or(0.0), b.as_f64().unwrap_or(0.0));
            if (a - b).abs() > 0.2 && (a - b).abs() > 0.05 * a.abs().max(b.abs()) {
                out.push(format!("{at}: golden {a}, now {b}"));
            }
        }
        (a, b) if a == b => {}
        (a, b) => out.push(format!("{at}: golden {a}, now {b}")),
    }
}
