//! One vehicle of the script rig: the scripted run and its report line.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::time::Instant;

use opts_bevy::host_callbacks::vehicle_callbacks;
use opts_sim::script::inputs::HostInputs;
use opts_sim::script::ScriptEngine;

use super::*;

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

/// The golden sample (`GOLDEN_VARS`) when `t` is one of the `GOLDEN_TIMES`.
fn golden_sample(engine: &ScriptEngine, t: f32) -> Option<serde_json::Value> {
    if !GOLDEN_TIMES.iter().any(|g| (t - g).abs() < DT * 0.5) {
        return None;
    }
    let values: serde_json::Map<String, serde_json::Value> = GOLDEN_VARS
        .iter()
        .map(|n| ((*n).to_owned(), serde_json::json!(engine.get(n))))
        .collect();
    Some(serde_json::json!({ "time": GOLDEN_TIMES.iter().copied().find(|g| (t - g).abs() < DT * 0.5), "values": values }))
}

/// One CSV row of every variable; the first frame writes the header and fixes the column order.
fn write_csv_row(
    w: &mut std::fs::File,
    names: &mut Vec<String>,
    frame: usize,
    t: f32,
    vars: &[(String, f32)],
) {
    if frame == 0 {
        *names = vars.iter().map(|(n, _)| n.clone()).collect();
        names.sort();
        let _ = writeln!(w, "time,{}", names.join(","));
    }
    let map: BTreeMap<&str, f32> = vars.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    let row: Vec<String> = names
        .iter()
        .map(|n| format!("{}", map.get(n.as_str()).copied().unwrap_or(0.0)))
        .collect();
    let _ = writeln!(w, "{t:.4},{}", row.join(","));
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

/// Adds one to the counter of every name.
fn bump<'a>(counts: &mut BTreeMap<String, usize>, names: impl IntoIterator<Item = &'a String>) {
    for n in names {
        *counts.entry(n.clone()).or_default() += 1;
    }
}

/// Folds one vehicle's run into the totals, writes its JSON line and checks the golden.
pub(super) fn report_vehicle(
    args: &Args,
    totals: &mut Totals,
    info: &VehicleInfo<'_>,
    engine: &ScriptEngine,
    run: VehicleRun,
    out: &mut Option<std::fs::File>,
) {
    let VehicleRun {
        plan,
        usage,
        undeclared,
        samples,
        runtime,
        non_finite,
        ops_total,
        micros_total,
    } = run;
    let frames = args.frames.max(1) as f64;
    totals.frame_us.push(micros_total / frames);

    let unfed_reads: BTreeSet<String> = usage
        .reads
        .iter()
        .filter(|n| !usage.writes.contains(*n) && plan.status(n).is_none())
        .filter(|n| undeclared.contains(*n))
        .cloned()
        .collect();
    let unfed_system: BTreeSet<String> = usage
        .system_reads
        .iter()
        .filter(|n| plan.status(n).is_none() && !ENGINE_SET.contains(&n.as_str()))
        .cloned()
        .collect();
    let stats = engine.callback_stats();
    let stub_names: BTreeSet<String> = stats.stub.keys().cloned().collect();
    let real_names: BTreeSet<String> = stats.real.keys().cloned().collect();
    let unknown_names: BTreeSet<String> = stats.unknown.keys().cloned().collect();
    let unconsumed_outputs = unconsumed(&usage.writes);
    let underflows = runtime.iter().filter(|m| m.contains("underflow")).count();
    totals.underflow_vehicles += usize::from(underflows > 0);
    totals.non_finite_vehicles += usize::from(!non_finite.is_empty());
    bump(&mut totals.unfed, unfed_reads.iter().chain(&unfed_system));
    bump(&mut totals.stubbed, &stub_names);
    bump(&mut totals.real, &real_names);
    bump(&mut totals.unconsumed_count, &unconsumed_outputs);
    let line = serde_json::json!({
        "vehicle": info.rel,
        "scripts": info.scripts,
        "load_errors": info.load_errors,
        "load_warnings": info.load_warnings,
        "unfed_inputs": unfed_reads,
        "unfed_system_values": unfed_system,
        "unconsumed_outputs": unconsumed_outputs,
        "callbacks_called": usage.external,
        "callbacks_real": real_names,
        "callbacks_stubbed": stub_names,
        "callbacks_unknown": unknown_names,
        "stack_underflows": underflows,
        "non_finite": non_finite,
        "ops_per_frame": ops_total as f64 / frames,
        "micros_per_frame": micros_total / frames,
        "runtime_messages": runtime.iter().filter(|m| !m.contains("neutral stub")).take(8).collect::<Vec<_>>(),
    });
    if let Some(o) = out.as_mut() {
        let _ = writeln!(o, "{line}");
    }
    write_or_check_golden(info.rel, &samples, args.golden.as_deref(), args.check.as_deref());
    if args.filter.is_some() {
        println!(
            "{}",
            serde_json::to_string_pretty(&line).unwrap_or_default()
        );
    }
}
