//! The transfer test (M-0103): a path recorded at the reference mass, run at another mass,
//! against the script run at that mass. Tests whether a recorded path, not a sampled table,
//! carries the gear state of a vehicle.

use std::path::Path;

use super::agreement::{
    has_ai_frame, FRAME_S, LIMIT_S, REF_MASS_KG, REF_RADIUS_M, TARGET_MS, TOLERANCE, WARM_UP_FRAMES,
};
use super::BusManifest;
use opts_kinematic::{run_up_with, DriveCurve};
use opts_sim::script::{load_vehicle_scripts, AiResponse, ScriptEngine};

use super::{find_ci, read_script};

/// Writes the vehicles that keep their script to the file named by `FALLBACK`, if set.
pub fn write_fallback(counts: &super::Counts) {
    if let Some(path) = std::env::var_os("FALLBACK") {
        let list: String = counts.fallback.iter().map(|n| format!("{n}\n")).collect();
        std::fs::write(&path, list).expect("write the fallback list");
        println!(
            "fallback list: {} vehicles keep their script, written to {}",
            counts.fallback.len(),
            path.to_string_lossy()
        );
    }
}

/// Mass (kg) of the transfer test: the path is recorded at the reference mass and run at this one.
const OTHER_MASS_KG: f32 = 8_000.0;

/// The vehicle's script engine after its standstill warm-up, with the AI flag it needs.
fn warmed(bus: &BusManifest, dir: &Path, root: &Path) -> (ScriptEngine, bool) {
    let (mut engine, _) = load_vehicle_scripts(bus, |rel| {
        read_script(find_ci(dir, rel).or_else(|| find_ci(root, rel)))
    });
    let ai = has_ai_frame(bus, dir, root);
    engine.set("AI", 1.0);
    engine.set("AI_Engine", 1.0);
    engine.run_init();
    for _ in 0..WARM_UP_FRAMES {
        drive(&mut engine, 0.0, FRAME_S, ai);
    }
    (engine, ai)
}

/// One frame of full throttle at `kmh`: the script's `M_Wheel`.
fn drive(engine: &mut ScriptEngine, kmh: f32, dt: f32, ai: bool) -> f32 {
    engine.set("Velocity", kmh);
    engine.set("Velocity_Ground", kmh);
    engine.set("Throttle", 1.0);
    engine.set("Brake", 0.0);
    engine.run_frame(dt, ai);
    engine.get("M_Wheel")
}

/// The torque (km/h, N·m) of every frame of a run-up from rest at `mass_kg`, as the script gives it.
fn path(bus: &BusManifest, dir: &Path, root: &Path, mass_kg: f32) -> Vec<(f32, f32)> {
    let (mut engine, ai) = warmed(bus, dir, root);
    let (mut v, mut t, mut pts) = (0.0_f32, 0.0_f32, Vec::new());
    while t < LIMIT_S {
        let torque = drive(&mut engine, v * 3.6, FRAME_S, ai);
        pts.push((v * 3.6, torque));
        v = (v + torque / (REF_RADIUS_M * mass_kg) * FRAME_S).max(0.0);
        t += FRAME_S;
        if v >= TARGET_MS {
            break;
        }
    }
    pts
}

/// Whether the path and the sampled table agree with the script at `OTHER_MASS_KG`; `None` when
/// both the script and the model stall.
pub fn transfer_check(
    bus: &BusManifest,
    dir: &Path,
    root: &Path,
    r: &AiResponse,
) -> (Option<bool>, Option<bool>) {
    let (mut engine, ai) = warmed(bus, dir, root);
    let run = |model: &dyn Fn(f32) -> f32| {
        run_up_with(
            |kmh, _| model(kmh),
            TARGET_MS,
            OTHER_MASS_KG,
            REF_RADIUS_M,
            FRAME_S,
            LIMIT_S,
        )
    };
    let script = run_up_with(
        |kmh, dt| drive(&mut engine, kmh, dt, ai),
        TARGET_MS,
        OTHER_MASS_KG,
        REF_RADIUS_M,
        FRAME_S,
        LIMIT_S,
    );
    let curve = DriveCurve::new(path(bus, dir, root, REF_MASS_KG));
    let by_path = run(&|kmh| curve.torque_at(kmh));
    let by_table = run(&|kmh| r.torque_at(kmh));
    let close = |model: Option<(f32, f32)>| match (script, model) {
        (None, None) => None,
        (Some((ts, _)), Some((tm, _))) => Some((tm - ts).abs() <= TOLERANCE * ts),
        _ => Some(false),
    };
    (close(by_path), close(by_table))
}

pub fn tally(c: &mut [usize; 3], ok: Option<bool>) {
    match ok {
        Some(true) => c[0] += 1,
        Some(false) => c[1] += 1,
        None => c[2] += 1,
    }
}
