//! The physics report and its golden values.

use super::phases::Loop;
use super::*;

mod scenarios;
use scenarios::{accel_scenario, brake_scenario, grade_scenario, sweep_scenario};

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
