//! One vehicle of the script rig: the scripted run and its report line.

use std::time::Instant;

use opts_bevy::host_callbacks::vehicle_callbacks;
use opts_sim::script::inputs::HostInputs;
use opts_sim::script::ScriptEngine;

use super::*;
use std::collections::BTreeMap;

mod report;
mod sample;
pub(super) use report::report_vehicle;
use sample::{golden_sample, write_csv_row};

/// What the rig counts over all vehicles.
#[derive(Default)]
pub(super) struct Totals {
    pub(super) with_scripts: usize,
    pub(super) underflow_vehicles: usize,
    pub(super) non_finite_vehicles: usize,
    pub(super) load_errors: usize,
    pub(super) unfed: BTreeMap<String, usize>,
    pub(super) stubbed: BTreeMap<String, usize>,
    pub(super) unconsumed_count: BTreeMap<String, usize>,
    pub(super) real: BTreeMap<String, usize>,
    pub(super) frame_us: Vec<f64>,
}

/// What the scripted run of one vehicle left behind.
pub(super) struct VehicleRun {
    plan: FeedPlan,
    usage: opts_sim::script::ScriptUsage,
    undeclared: BTreeSet<String>,
    samples: Vec<serde_json::Value>,
    runtime: Vec<String>,
    non_finite: BTreeSet<String>,
    ops_total: u64,
    micros_total: f64,
}

/// What the load of a vehicle's scripts reported.
pub(super) struct VehicleInfo<'a> {
    pub(super) rel: &'a str,
    pub(super) scripts: usize,
    pub(super) load_errors: usize,
    pub(super) load_warnings: usize,
}

/// Runs the scripts of one vehicle for `args.frames` frames under the scripted input profile.
pub(super) fn run_vehicle(
    args: &Args,
    f: &Path,
    rel: &str,
    engine: &mut ScriptEngine,
) -> VehicleRun {
    let seed = seed_of(rel);
    engine.seed_random(seed as u32);
    engine.set_external_handler(vehicle_callbacks(seed));
    let plan = FeedPlan::new(engine);
    let mut inputs = HostInputs::default();
    let usage = engine.usage();
    let undeclared: BTreeSet<String> = engine
        .undeclared_variables()
        .into_iter()
        .map(|n| n.to_lowercase())
        .collect();
    let mut csv_out = args
        .csv
        .as_ref()
        .filter(|_| args.filter.is_some())
        .and_then(|d| {
            let name = f.file_stem()?.to_string_lossy().into_owned();
            std::fs::File::create(d.join(format!("{name}.csv"))).ok()
        });
    let mut speed_ms = 0.0f32;
    let mut startup = Startup::default();
    let mut run = VehicleRun {
        plan,
        usage,
        undeclared,
        samples: Vec::new(),
        runtime: Vec::new(),
        non_finite: BTreeSet::new(),
        ops_total: 0,
        micros_total: 0.0,
    };
    feed(engine, &run.plan, &inputs);
    engine.run_init();
    let mut csv_names: Vec<String> = Vec::new();
    for frame in 0..args.frames {
        let t = frame as f32 * DT;
        inputs.throttle = if (1.0..6.0).contains(&t) { 0.7 } else { 0.0 };
        inputs.brake = if (8.0..10.0).contains(&t) { 0.5 } else { 0.0 };
        speed_ms = (speed_ms + (inputs.throttle * 2.0 - inputs.brake * 4.0 - 0.1) * DT).max(0.0);
        inputs.velocity_km_h = speed_ms * 3.6;
        inputs.velocity_ground_km_h = inputs.velocity_km_h;
        inputs.n_wheel_rpm = speed_ms / (std::f32::consts::PI * 0.98) * 60.0;
        inputs.time_of_day_s = 8.0 * 3600.0 + t;
        feed(engine, &run.plan, &inputs);
        startup.step(t, engine);
        let (ops0, t0) = (engine.ops_executed(), Instant::now());
        engine.run_frame(DT, false);
        run.samples.extend(golden_sample(engine, t));
        run.micros_total += t0.elapsed().as_secs_f64() * 1e6;
        run.ops_total += engine.ops_executed() - ops0;
        let vars = engine.variables();
        for (n, v) in &vars {
            if !v.is_finite() {
                run.non_finite.insert(n.clone());
            }
        }
        if let Some(w) = csv_out.as_mut() {
            write_csv_row(w, &mut csv_names, frame, t, &vars);
        }
        run.runtime
            .extend(engine.drain_diagnostics().into_iter().map(|d| d.message));
    }
    run
}
