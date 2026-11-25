//! Tests of the measured AI response (M-0103): the shared car law, the interpolation, and
//! the warm-up that lets a slow engine reach its working state before it is sampled.

use opts_sim::script::{measure_ai_response, AiResponse, ScriptEngine};

const SHARED_CAR_SCRIPT: &str = "{frame}\n\
(L.L.Throttle) (C.L.AI_M_gas_max) * 1 (L.L.Velocity) (C.L.AI_v_max_erreich) / - * (S.L.M_Wheel)\n\
(L.L.Brake) (C.L.AI_F_brems_max) * (S.L.Brakeforce)\n{end}\n";

fn engine() -> ScriptEngine {
    let (mut e, diags) = ScriptEngine::from_sources(&[("main_AI.osc", SHARED_CAR_SCRIPT)]);
    assert!(diags.is_empty(), "{diags:?}");
    e.consts_mut().set_scalar("AI_M_gas_max", 1000.0);
    e.consts_mut().set_scalar("AI_v_max_erreich", 190.0);
    e.consts_mut().set_scalar("AI_F_brems_max", 18_000.0);
    e
}

#[test]
fn the_shared_car_script_follows_its_law() {
    let r = measure_ai_response(&mut engine());
    assert!((r.torque_at(0.0) - 1000.0).abs() < 1e-3);
    let want = 1000.0 * (1.0 - 50.0 / 190.0);
    assert!(
        (r.torque_at(50.0) - want).abs() < 0.5,
        "{}",
        r.torque_at(50.0)
    );
    assert_eq!(r.brake_force, 18_000.0);
    assert!(r.is_live());
}

#[test]
fn a_slow_engine_is_measured_after_it_has_spun_up() {
    // The torque ramps by 0.02 per frame, so it reaches its 1000 N·m only after 50 frames:
    // a measurement of 8 frames would read 160 N·m.
    let slow = "{frame}\n\
(L.L.ramp) 0.02 + (S.L.ramp)\n\
(L.L.ramp) 1 min 1000 * (S.L.M_Wheel)\n{end}\n";
    let (mut e, diags) = ScriptEngine::from_sources(&[("main_AI.osc", slow)]);
    assert!(diags.is_empty(), "{diags:?}");
    let r = measure_ai_response(&mut e);
    assert!(
        (r.torque_at(0.0) - 1000.0).abs() < 1e-3,
        "{}",
        r.torque_at(0.0)
    );
}

#[test]
fn interpolation_clamps_and_is_linear() {
    let r = AiResponse {
        drive: vec![(0.0, 100.0), (10.0, 50.0), (20.0, 0.0)],
        brake_force: 0.0,
    };
    assert_eq!(r.torque_at(-5.0), 100.0);
    assert_eq!(r.torque_at(5.0), 75.0);
    assert_eq!(r.torque_at(15.0), 25.0);
    assert_eq!(r.torque_at(99.0), 0.0);
    assert_eq!(r.peak_torque(), 100.0);
    assert!(r.is_live());
    assert!(!AiResponse {
        drive: vec![(0.0, 0.0)],
        brake_force: 0.0
    }
    .is_live());
}
