//! The census measured with the inputs the game feeds to a vehicle's script (M-0103 input
//! completeness, `FULL=1`): wheel speed, wear and the pedals, through the same feed plan. The
//! plain census sets only the speed and the pedals, so a lorry's gearbox never sees its wheels turn.

use std::f32::consts::PI;

use opts_sim::script::inputs::{feed, FeedPlan, HostInputs, WEARLIFESPAN_NO_BREAKDOWNS};
use opts_sim::script::{AiResponse, ScriptEngine};

use super::agreement::{FRAME_S, REF_MASS_KG, REF_RADIUS_M, WARM_UP_FRAMES};

/// Speeds (km/h) at which the drive torque is sampled, as `measure_ai_response` does.
const SAMPLE_KMH: [f32; 8] = [0.0, 5.0, 10.0, 20.0, 30.0, 50.0, 70.0, 100.0];
/// Frames per sample, as `measure_ai_response` does.
const SETTLE_FRAMES: usize = 8;

/// Whether the census is measured with the game's inputs (`FULL` is set).
pub fn enabled() -> bool {
    std::env::var_os("FULL").is_some()
}

/// One frame with the game's inputs at `kmh` and the given pedals.
fn step(engine: &mut ScriptEngine, plan: &FeedPlan, kmh: f32, pedals: (f32, f32), ai: bool) {
    let rpm = kmh / 3.6 / REF_RADIUS_M * 60.0 / (2.0 * PI);
    let inputs = HostInputs {
        velocity_km_h: kmh,
        velocity_ground_km_h: kmh,
        n_wheel_rpm: rpm,
        throttle: pedals.0,
        brake: pedals.1,
        ai,
        ai_engine: 1.0,
        wearlifespan: WEARLIFESPAN_NO_BREAKDOWNS,
        ..HostInputs::default()
    };
    feed(engine, plan, &inputs);
    engine.run_frame(FRAME_S, ai);
}

/// The response of `measure_ai_response`, with the game's inputs fed each frame.
pub fn measure(engine: &mut ScriptEngine, ai_blocks: bool, name: &str) -> AiResponse {
    let trace = std::env::var("TRACE")
        .ok()
        .filter(|t| name.contains(t.as_str()))
        .is_some();
    engine.set("AI", 1.0);
    engine.set("AI_Engine", 1.0);
    engine.run_init();
    let plan = FeedPlan::new(engine);
    for _ in 0..WARM_UP_FRAMES {
        step(engine, &plan, 0.0, (1.0, 0.0), ai_blocks);
    }
    let drive = SAMPLE_KMH
        .iter()
        .map(|&kmh| {
            for _ in 0..SETTLE_FRAMES {
                step(engine, &plan, kmh, (1.0, 0.0), ai_blocks);
            }
            if trace {
                eprintln!("TRACE {name} kmh={kmh} M_Wheel={} engine_n={} gang={} ratio={} clutch={} engine_M={} wf={}", engine.get("M_Wheel"), engine.get("engine_n"), engine.get("antrieb_getr_gang"), engine.get("antrieb_getr_ratio_act"), engine.get("AI_clutchtimer"), engine.get("engine_M"), engine.get("Wheel_RotationSpeed_1_R"));
            }
            (kmh, engine.get("M_Wheel"))
        })
        .collect();
    for _ in 0..SETTLE_FRAMES {
        step(engine, &plan, 30.0, (0.0, 1.0), ai_blocks);
    }
    let brake_force = engine.get("Brakeforce");
    AiResponse { drive, brake_force }
}

/// Speed (m/s) at which a run-up counts as reached, and its frame limit.
const RUN_TARGET_MS: f32 = 100.0 / 3.6;
const RUN_LIMIT_FRAMES: usize = 60 * 120;

/// The response as the torque of a continuous run-up from rest at the reference mass, one point
/// per frame (speed, `M_Wheel`), with the game's inputs. Speed jumps between samples break the
/// gearbox's speed-driven shifts; a run-up does not.
pub fn measure_path(engine: &mut ScriptEngine, ai_blocks: bool, name: &str) -> AiResponse {
    let trace = std::env::var("TRACE")
        .ok()
        .filter(|t| name.contains(t.as_str()))
        .is_some();
    engine.set("AI", 1.0);
    engine.set("AI_Engine", 1.0);
    engine.run_init();
    let plan = FeedPlan::new(engine);
    for _ in 0..WARM_UP_FRAMES {
        step(engine, &plan, 0.0, (1.0, 0.0), ai_blocks);
    }
    let (mut v, mut drive) = (0.0_f32, Vec::new());
    for frame in 0..RUN_LIMIT_FRAMES {
        let kmh = v * 3.6;
        step(engine, &plan, kmh, (1.0, 0.0), ai_blocks);
        let torque = engine.get("M_Wheel");
        if trace && frame % 120 == 0 {
            eprintln!(
                "PATH {name} t={:.1}s kmh={kmh:.1} M_Wheel={torque} gang={}",
                frame as f32 / 60.0,
                engine.get("antrieb_getr_gang")
            );
        }
        drive.push((kmh, torque));
        v = (v + torque / (REF_RADIUS_M * REF_MASS_KG) * FRAME_S).max(0.0);
        if v >= RUN_TARGET_MS {
            break;
        }
    }
    for _ in 0..SETTLE_FRAMES {
        step(engine, &plan, 30.0, (0.0, 1.0), ai_blocks);
    }
    AiResponse {
        drive,
        brake_force: engine.get("Brakeforce"),
    }
}

/// The response of the mode the environment selects: `PATH_TABLE` (run-up path), `FULL` (the
/// game's inputs, speed samples), or none (`None`: the plain `measure_ai_response`).
pub fn response(engine: &mut ScriptEngine, ai: bool, name: &str) -> Option<AiResponse> {
    if std::env::var_os("PATH_TABLE").is_some() {
        Some(measure_path(engine, ai, name))
    } else if enabled() {
        Some(measure(engine, ai, name))
    } else {
        None
    }
}
