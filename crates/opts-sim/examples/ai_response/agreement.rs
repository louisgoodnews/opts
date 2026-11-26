//! Agreement of the cheap table model with the script (M-0103 criterion 2): the closed-loop
//! time to 50 km/h of the script and of the measured table, with the same mass and radius.

use std::path::Path;

use super::BusManifest;
use opts_kinematic::run_up_with;
use opts_sim::script::{load_vehicle_scripts, AiResponse, ScriptEngine};

use super::{find_ci, read_script};

/// Reference mass (kg) and wheel radius (m): both models scale by the same factor, so the
/// agreement does not depend on them.
pub const REF_MASS_KG: f32 = 12_000.0;
pub const REF_RADIUS_M: f32 = 0.5;
/// Frame time, run-up target (50 km/h in m/s), run-up limit (s) and tolerance: the same frame
/// count as the measurement's warm-up.
pub const FRAME_S: f32 = 1.0 / 60.0;
pub const TARGET_MS: f32 = 50.0 / 3.6;
pub const LIMIT_S: f32 = 120.0;
pub const TOLERANCE: f32 = 0.15;
pub const WARM_UP_FRAMES: usize = 600;

/// Whether the vehicle's scripts have `{frame_ai}` blocks (what `measure_ai_response` checks).
pub fn has_ai_frame(bus: &BusManifest, dir: &Path, root: &Path) -> bool {
    bus.scripts
        .iter()
        .filter_map(|rel| find_ci(dir, rel).or_else(|| find_ci(root, rel)))
        .filter_map(|p| std::fs::read(p).ok())
        .any(|b| {
            String::from_utf8_lossy(&b)
                .to_lowercase()
                .contains("{frame_ai}")
        })
}

/// Closed-loop run-up of the vehicle's own script: each frame it gets the current speed and
/// its `M_Wheel` drives the speed, warmed up at standstill as the measurement is.
fn scripted_run_up(bus: &BusManifest, dir: &Path, root: &Path) -> Option<(f32, f32)> {
    let (mut engine, _) = load_vehicle_scripts(bus, |rel| {
        read_script(find_ci(dir, rel).or_else(|| find_ci(root, rel)))
    });
    let ai = has_ai_frame(bus, dir, root);
    engine.set("AI", 1.0);
    engine.set("AI_Engine", 1.0);
    engine.run_init();
    let drive = |engine: &mut ScriptEngine, kmh: f32, dt: f32| -> f32 {
        engine.set("Velocity", kmh);
        engine.set("Velocity_Ground", kmh);
        engine.set("Throttle", 1.0);
        engine.set("Brake", 0.0);
        engine.run_frame(dt, ai);
        engine.get("M_Wheel")
    };
    for _ in 0..WARM_UP_FRAMES {
        drive(&mut engine, 0.0, FRAME_S);
    }
    run_up_with(
        |kmh, dt| drive(&mut engine, kmh, dt),
        TARGET_MS,
        REF_MASS_KG,
        REF_RADIUS_M,
        FRAME_S,
        LIMIT_S,
    )
}

/// Compares the closed-loop time to 50 km/h of the script and of the measured table.
pub fn check_agreement(
    name: &str,
    bus: &BusManifest,
    dir: &Path,
    root: &Path,
    r: &AiResponse,
    show: bool,
    counts: &mut Counts,
) {
    let table = run_up_with(
        |kmh, _| r.torque_at(kmh),
        TARGET_MS,
        REF_MASS_KG,
        REF_RADIUS_M,
        FRAME_S,
        LIMIT_S,
    );
    let Some(script) = scripted_run_up(bus, dir, root) else {
        match table {
            None => counts.stall += 1,
            Some(_) => {
                counts.disagree += 1;
                counts.fallback.insert(name.to_string());
                if show {
                    println!("NORUN     {name} (table reaches 50 km/h, script does not)");
                }
            }
        }
        return;
    };
    match table {
        Some((t, _)) if (t - script.0).abs() <= TOLERANCE * script.0 => counts.agree += 1,
        other => {
            counts.disagree += 1;
            counts.fallback.insert(name.to_string());
            if show {
                println!(
                    "DISAGREE  {name} | script {:.1} s, table {:?}",
                    script.0,
                    other.map(|o| o.0)
                );
            }
        }
    }
}

#[derive(Default)]
pub struct Counts {
    pub total: usize,
    pub no_script: usize,
    pub dead: usize,
    pub follows: usize,
    pub deviates: usize,
    pub agree: usize,
    pub disagree: usize,
    pub stall: usize,
    pub no_run: usize,
    /// Transfer test: [agree, disagree, stall in both] for the path and for the sampled table.
    pub path: [usize; 3],
    pub table: [usize; 3],
    /// Vehicles that keep their script: they fail the agreement or the path transfer.
    pub fallback: std::collections::BTreeSet<String>,
}
