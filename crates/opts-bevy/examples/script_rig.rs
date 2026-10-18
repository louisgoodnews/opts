//! The script rig (M-0096): runs the scripts of a vehicle (or of every bus of an installation)
//! headless under a scripted input profile and writes the **integration report**: what the
//! scripts need that the host does not supply, what they produce that nothing consumes, which
//! callbacks were answered by the host, by a stub or not at all, and whether anything went wrong
//! (compile and run-time diagnostics, stack underflows, non-finite values, cost per frame).
//!
//! `script_rig <OMSI root> [--vehicle <text in the .bus path>] [--frames N] [--json FILE]
//! [--csv DIR] [--limit N] [--golden FILE | --check FILE]`
//!
//! The profile (60 frames per second): the driver switches the battery on at 0.5 s, starts the
//! engine once the busbar has voltage and selects *Drive* (the triggers of the game's start-up,
//! `vehicle_script::script_startup_system`); then the driver then releases the parking brake; rest until the first second, 70 % throttle until
//! the sixth, coasting, a 50 % service brake from the eighth to the tenth second; the speed
//! follows from a crude longitudinal model (the real physics is not part of the rig). `--json`
//! writes one line per vehicle, `--csv DIR` one CSV of every non-zero float variable per frame for
//! each vehicle the filter selects (use it with `--vehicle`).
//!
//! `--physics` closes the loop instead: the scripts' torque and brake force drive a
//! `VehicleSimulation` (script drive) at `--hz` steps per second, and the rig measures what a
//! vehicle does under its own scripts: the time from standstill to 50 km/h at full throttle, the
//! distance to stop from 50 km/h at full brake, whether the parking brake holds on a 10 % grade
//! and whether more throttle gives more speed (`docs/src/architecture/script-integration-report.md`,
//! M-0097).
//!
//! `--golden FILE` writes the values of [`GOLDEN_VARS`] at [`GOLDEN_TIMES`] for the vehicle the
//! filter selects; `--check FILE` runs the same and exits with 1 when a value differs from the
//! golden by more than 2 % (or 0.001 absolute). The golden run of the MAN NG272 (`MAN_GN92_main`)
//! is `crates/opts-bevy/tests/golden/script-rig-gn92.json`; it needs the installation, so it is a
//! local gate, not a CI test.
//!
//! The rig is the verification surface of the script programme
//! (`docs/src/architecture/bus-script-integration.md`, principle 8): each later phase is accepted
//! by a number of this report.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use opts_bevy::host_callbacks::vehicle_callbacks;
use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::inputs::{feed, FeedPlan, HostInputs, OUTPUT_TABLE};
use opts_sim::script::{load_vehicle_scripts, Severity};

const DT: f32 = 1.0 / 60.0;

/// Variables of a golden run, sampled at [`GOLDEN_TIMES`] (seconds).
const GOLDEN_VARS: &[&str] = &[
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
const GOLDEN_TIMES: &[f32] = &[3.0, 6.0, 9.0, 11.0];

/// The driver's start-up as the game performs it (battery switch, engine start key, *Drive*).
#[derive(Default)]
struct Startup {
    stage: u8,
    mark: f32,
}

impl Startup {
    fn step(&mut self, t: f32, engine: &mut opts_sim::script::ScriptEngine) {
        match self.stage {
            0 if t > 0.5 => {
                engine.trigger("cp_batterietrennschalter_toggle");
                (self.stage, self.mark) = (1, t);
            }
            1 if t - self.mark > 0.3 => {
                engine.trigger("cp_batterietrennschalter_toggle_off");
                (self.stage, self.mark) = (2, t);
            }
            2 if engine.get("elec_busbar_main") > 0.3 && t - self.mark > 1.0 => {
                engine.trigger("kw_m_enginestart");
                (self.stage, self.mark) = (3, t);
            }
            3 if engine.get("engine_on") > 0.5 || t - self.mark > 8.0 => {
                engine.trigger("kw_m_enginestart_off");
                // The automatic leaves neutral only with the brake pressed.
                let brake = engine.get("brake");
                engine.set("brake", 1.0);
                engine.trigger("automatic_d");
                engine.set("brake", brake);
                (self.stage, self.mark) = (4, t);
            }
            // The driver releases the parking brake the scripts start with applied.
            4 if t - self.mark > 0.5 => {
                if engine.get("bremse_feststell") > 0.5 {
                    engine.trigger("parking_brake_toggle");
                }
                (self.stage, self.mark) = (5, t);
            }
            _ => {}
        }
    }
}

/// A vehicle's scripts and physics, closed in a loop (`--physics`).
struct Loop {
    engine: opts_sim::script::ScriptEngine,
    plan: FeedPlan,
    controller: opts_bevy::VehicleController,
    startup: Startup,
    t: f32,
    dt: f32,
}

impl Loop {
    fn new(bus: &BusManifest, engine: opts_sim::script::ScriptEngine, dt: f32) -> Self {
        let mut engine = engine;
        let plan = FeedPlan::new(&mut engine);
        let mut controller =
            opts_bevy::VehicleController::new(opts_sim::VehicleSimulation::from_manifest(bus));
        controller
            .inputs_mut()
            .set_gear(opts_sim::GearPosition::Drive);
        controller.inputs_mut().set_parking_brake(true);
        let first = opts_bevy::vehicle_script::host_inputs(&controller, None);
        feed(&mut engine, &plan, &first);
        engine.run_init();
        Self {
            engine,
            plan,
            controller,
            startup: Startup::default(),
            t: 0.0,
            dt,
        }
    }

    /// One step: feed, start-up triggers, scripts, their torque and brakes into the physics.
    fn tick(&mut self) {
        let inputs = opts_bevy::vehicle_script::host_inputs(&self.controller, None);
        feed(&mut self.engine, &self.plan, &inputs);
        self.startup.step(self.t, &mut self.engine);
        self.engine.run_frame(self.dt, false);
        self.controller.sim.script_drive = opts_bevy::vehicle_script::script_outputs(&self.engine);
        self.controller.sim.step(self.dt);
        self.t += self.dt;
    }

    fn run(&mut self, seconds: f32) {
        let trace = std::env::var_os("RIG_TRACE").is_some();
        for i in 0..(seconds / self.dt) as usize {
            self.tick();
            if trace && i % (1.0 / self.dt) as usize == 0 {
                eprintln!(
                    "t={:.1} v={:.2} engine_on={} n={:.0} m_wheel={:.0} brake={:.0} park={} gear={}",
                    self.t,
                    self.speed(),
                    self.engine.get("engine_on"),
                    self.engine.get("engine_n"),
                    self.engine.get("m_wheel"),
                    self.controller
                        .sim
                        .script_drive
                        .map_or(0.0, |d| d.brake_force_n),
                    self.engine.get("bremse_feststell"),
                    self.engine.get("antrieb_getr_aktugang"),
                );
            }
        }
    }

    fn speed(&self) -> f32 {
        self.controller.sim.chassis.forward_speed_m_s()
    }

    /// Start-up, then the parking brake off and the brake circuits filled until the scripts'
    /// brakes are released (the compressor needs up to two minutes), at most 180 s: the vehicle
    /// stands ready to drive.
    fn ready(&mut self) {
        self.run(12.0);
        self.controller.inputs_mut().set_parking_brake(false);
        if self.engine.get("bremse_feststell") > 0.5 {
            self.engine.trigger("parking_brake_toggle");
        }
        self.run(1.0);
        let t_limit = self.t + 170.0;
        while self.t < t_limit {
            let drag = self
                .controller
                .sim
                .script_drive
                .map_or(0.0, |d| d.brake_force_n);
            if drag < 500.0 {
                break;
            }
            self.run(1.0);
        }
    }
}

/// The closed-loop scenarios of one vehicle (`--physics`).
fn physics_report(
    bus: &BusManifest,
    load: &dyn Fn() -> opts_sim::script::ScriptEngine,
    hz: f32,
) -> serde_json::Value {
    let dt = 1.0 / hz;
    let fresh = || Loop::new(bus, load(), dt);
    // 1. Standstill to 50 km/h at full throttle.
    let mut a = fresh();
    a.ready();
    a.controller.inputs_mut().set_throttle(1.0);
    let t0 = a.t;
    let mut to_50 = None;
    while a.t - t0 < 40.0 {
        a.tick();
        if to_50.is_none() && a.speed() * 3.6 >= 50.0 {
            to_50 = Some(a.t - t0);
        }
    }
    let speed_after_40 = a.speed() * 3.6;
    // 2. Stop from 50 km/h at full brake.
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
            eprintln!(
                "brake t={:.2} v={:.2} force={:.0} pressure={:.0}",
                b.t - t1,
                b.speed(),
                b.controller
                    .sim
                    .script_drive
                    .map_or(0.0, |d| d.brake_force_n),
                b.engine.get("bremse_p_tank01"),
            );
        }
    }
    let stop_m = b.controller.sim.chassis.position.y - start;
    let stop_s = b.t - t1;
    // 3. Parking brake on a 10 % grade, facing uphill.
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
    // 4. A throttle sweep.
    let sweep: Vec<f32> = [0.2f32, 0.4, 0.6, 0.8, 1.0]
        .iter()
        .map(|&th| {
            let mut s = fresh();
            s.ready();
            s.controller.inputs_mut().set_throttle(th);
            s.run(8.0);
            s.speed() * 3.6
        })
        .collect();
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

/// System values the engine itself sets in every `run_frame` (`GetTime`, `Timegap`): never unfed.
const ENGINE_SET: &[&str] = &["gettime", "timegap"];

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
        if comp == ".." {
            cur = cur.parent()?.to_path_buf();
            continue;
        }
        let direct = cur.join(comp);
        if direct.exists() {
            cur = direct;
            continue;
        }
        let hit = std::fs::read_dir(&cur)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(comp))?;
        cur = hit.path();
    }
    Some(cur)
}

fn buses(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            buses(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bus")) {
            out.push(p);
        }
    }
}

/// A stable seed from the vehicle's path (FNV-1a).
fn seed_of(path: &str) -> u64 {
    path.bytes().fold(0xCBF2_9CE4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
    })
}

/// The output rows the scripts wrote that OPTS does not read yet.
fn unconsumed(writes: &BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for w in writes {
        for row in OUTPUT_TABLE {
            let hit = if row.name.ends_with('_') {
                w.starts_with(row.name)
            } else {
                w == row.name
            };
            if hit && !row.consumed {
                out.insert(row.name.to_owned());
            }
        }
    }
    out
}

/// Compares two physics reports; numbers may differ by 5 % (or 0.2 absolute: stopping and
/// hold distances are small), `hz` is not compared (the golden is for the step size it names).
fn physics_diff(
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

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("OMSI root"));
    let (mut filter, mut frames, mut json, mut csv, mut limit) = (
        None::<String>,
        720usize,
        None::<PathBuf>,
        None::<PathBuf>,
        usize::MAX,
    );
    let (mut golden, mut check) = (None::<PathBuf>, None::<PathBuf>);
    let (mut physics, mut hz) = (false, 60.0f32);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--vehicle" => filter = args.next().map(|s| s.to_lowercase()),
            "--frames" => frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(720),
            "--json" => json = args.next().map(PathBuf::from),
            "--csv" => csv = args.next().map(PathBuf::from),
            "--physics" => physics = true,
            "--hz" => hz = args.next().and_then(|v| v.parse().ok()).unwrap_or(60.0),
            "--golden" => golden = args.next().map(PathBuf::from),
            "--check" => check = args.next().map(PathBuf::from),
            "--limit" => {
                limit = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(usize::MAX)
            }
            other => panic!("unknown argument {other}"),
        }
    }
    let mut files = Vec::new();
    buses(&root.join("Vehicles"), &mut files);
    files.sort();
    let mut out = json.and_then(|p| std::fs::File::create(p).ok());
    if let Some(dir) = &csv {
        let _ = std::fs::create_dir_all(dir);
    }

    let mut with_scripts = 0usize;
    let mut underflow_vehicles = 0usize;
    let mut non_finite_vehicles = 0usize;
    let mut load_errors = 0usize;
    let mut unfed: BTreeMap<String, usize> = BTreeMap::new();
    let mut stubbed: BTreeMap<String, usize> = BTreeMap::new();
    let mut unconsumed_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut real: BTreeMap<String, usize> = BTreeMap::new();
    let mut frame_us = Vec::new();

    for f in &files {
        let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        if filter
            .as_ref()
            .is_some_and(|x| !rel.to_lowercase().contains(x))
        {
            continue;
        }
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let Ok(bus) = BusManifest::parse_bytes(&bytes) else {
            continue;
        };
        if bus.scripts.is_empty() {
            continue;
        }
        if with_scripts >= limit {
            break;
        }
        with_scripts += 1;
        let dir = f.parent().expect("dir").to_path_buf();
        let (mut engine, load) =
            load_vehicle_scripts(&bus, |r| find_ci(&dir, r).or_else(|| find_ci(&root, r)));
        let load_errors_here = load
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        load_errors += usize::from(load_errors_here > 0);
        if physics {
            let seed0 = seed_of(&rel);
            let loader = || {
                let (mut e, _) =
                    load_vehicle_scripts(&bus, |r| find_ci(&dir, r).or_else(|| find_ci(&root, r)));
                e.seed_random(seed0 as u32);
                e.set_external_handler(vehicle_callbacks(seed0));
                e
            };
            let report = physics_report(&bus, &loader, hz);
            println!(
                "{rel}\n{}",
                serde_json::to_string_pretty(&report).unwrap_or_default()
            );
            let value = serde_json::to_value(&report).unwrap_or_default();
            if let Some(path) = &golden {
                let text = serde_json::to_string_pretty(&value).unwrap_or_default();
                std::fs::write(path, format!("{text}\n")).expect("write the golden");
                println!("golden written to {}", path.display());
            }
            if let Some(path) = &check {
                let text = std::fs::read_to_string(path).expect("read the golden");
                let want: serde_json::Value =
                    serde_json::from_str(&text).expect("parse the golden");
                let mut diffs = Vec::new();
                physics_diff("", &want, &value, &mut diffs);
                for d in &diffs {
                    eprintln!("DIFFERS {d}");
                }
                if !diffs.is_empty() {
                    std::process::exit(1);
                }
                println!("physics golden holds ({})", path.display());
            }
            continue;
        }
        let seed = seed_of(&rel);
        engine.seed_random(seed as u32);
        engine.set_external_handler(vehicle_callbacks(seed));
        let plan = FeedPlan::new(&mut engine);
        let mut inputs = HostInputs::default();
        let usage = engine.usage();
        let undeclared: BTreeSet<String> = engine
            .undeclared_variables()
            .into_iter()
            .map(|n| n.to_lowercase())
            .collect();

        let mut csv_out = csv.as_ref().filter(|_| filter.is_some()).and_then(|d| {
            let name = f.file_stem()?.to_string_lossy().into_owned();
            std::fs::File::create(d.join(format!("{name}.csv"))).ok()
        });
        let mut speed_ms = 0.0f32;
        let mut startup = Startup::default();
        let mut samples: Vec<serde_json::Value> = Vec::new();
        feed(&mut engine, &plan, &inputs);
        engine.run_init();
        let mut runtime: Vec<String> = Vec::new();
        let mut non_finite: BTreeSet<String> = BTreeSet::new();
        let (mut ops_total, mut micros_total) = (0u64, 0.0f64);
        let mut csv_names: Vec<String> = Vec::new();
        for frame in 0..frames {
            let t = frame as f32 * DT;
            inputs.throttle = if (1.0..6.0).contains(&t) { 0.7 } else { 0.0 };
            inputs.brake = if (8.0..10.0).contains(&t) { 0.5 } else { 0.0 };
            speed_ms =
                (speed_ms + (inputs.throttle * 2.0 - inputs.brake * 4.0 - 0.1) * DT).max(0.0);
            inputs.velocity_km_h = speed_ms * 3.6;
            inputs.velocity_ground_km_h = inputs.velocity_km_h;
            inputs.n_wheel_rpm = speed_ms / (std::f32::consts::PI * 0.98) * 60.0;
            inputs.time_of_day_s = 8.0 * 3600.0 + t;
            feed(&mut engine, &plan, &inputs);
            startup.step(t, &mut engine);
            let (ops0, t0) = (engine.ops_executed(), Instant::now());
            engine.run_frame(DT, false);
            if GOLDEN_TIMES.iter().any(|g| (t - g).abs() < DT * 0.5) {
                let values: serde_json::Map<String, serde_json::Value> = GOLDEN_VARS
                    .iter()
                    .map(|n| ((*n).to_owned(), serde_json::json!(engine.get(n))))
                    .collect();
                samples.push(serde_json::json!({ "time": GOLDEN_TIMES.iter().copied().find(|g| (t - g).abs() < DT * 0.5), "values": values }));
            }
            micros_total += t0.elapsed().as_secs_f64() * 1e6;
            ops_total += engine.ops_executed() - ops0;
            let vars = engine.variables();
            for (n, v) in &vars {
                if !v.is_finite() {
                    non_finite.insert(n.clone());
                }
            }
            if let Some(w) = csv_out.as_mut() {
                if frame == 0 {
                    csv_names = vars.iter().map(|(n, _)| n.clone()).collect();
                    csv_names.sort();
                    let _ = writeln!(w, "time,{}", csv_names.join(","));
                }
                let map: BTreeMap<&str, f32> = vars.iter().map(|(n, v)| (n.as_str(), *v)).collect();
                let row: Vec<String> = csv_names
                    .iter()
                    .map(|n| format!("{}", map.get(n.as_str()).copied().unwrap_or(0.0)))
                    .collect();
                let _ = writeln!(w, "{t:.4},{}", row.join(","));
            }
            runtime.extend(engine.drain_diagnostics().into_iter().map(|d| d.message));
        }
        frame_us.push(micros_total / frames.max(1) as f64);

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
        underflow_vehicles += usize::from(underflows > 0);
        non_finite_vehicles += usize::from(!non_finite.is_empty());
        for n in unfed_reads.iter().chain(&unfed_system) {
            *unfed.entry(n.clone()).or_default() += 1;
        }
        for n in &stub_names {
            *stubbed.entry(n.clone()).or_default() += 1;
        }
        for n in &real_names {
            *real.entry(n.clone()).or_default() += 1;
        }
        for n in &unconsumed_outputs {
            *unconsumed_count.entry(n.clone()).or_default() += 1;
        }
        let line = serde_json::json!({
            "vehicle": rel,
            "scripts": bus.scripts.len(),
            "load_errors": load_errors_here,
            "load_warnings": load.iter().filter(|d| d.severity == Severity::Warning).count(),
            "unfed_inputs": unfed_reads,
            "unfed_system_values": unfed_system,
            "unconsumed_outputs": unconsumed_outputs,
            "callbacks_called": usage.external,
            "callbacks_real": real_names,
            "callbacks_stubbed": stub_names,
            "callbacks_unknown": unknown_names,
            "stack_underflows": underflows,
            "non_finite": non_finite,
            "ops_per_frame": ops_total as f64 / frames.max(1) as f64,
            "micros_per_frame": micros_total / frames.max(1) as f64,
            "runtime_messages": runtime.iter().filter(|m| !m.contains("neutral stub")).take(8).collect::<Vec<_>>(),
        });
        if let Some(o) = out.as_mut() {
            let _ = writeln!(o, "{line}");
        }
        let golden_doc =
            serde_json::json!({ "vehicle": rel, "frames_per_second": 60, "samples": samples });
        if let Some(path) = &golden {
            std::fs::write(
                path,
                serde_json::to_string_pretty(&golden_doc).unwrap_or_default(),
            )
            .expect("write the golden file");
            println!("golden run of {rel} written to {}", path.display());
        }
        if let Some(path) = &check {
            let want: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).expect("read the golden file"))
                    .expect("golden file is JSON");
            let mut bad = Vec::new();
            for (w, g) in want["samples"]
                .as_array()
                .into_iter()
                .flatten()
                .zip(golden_doc["samples"].as_array().into_iter().flatten())
            {
                for name in GOLDEN_VARS {
                    let (a, b) = (
                        w["values"][name].as_f64().unwrap_or(f64::NAN),
                        g["values"][name].as_f64().unwrap_or(f64::NAN),
                    );
                    if (a - b).abs() > 0.001 + 0.02 * a.abs() {
                        bad.push(format!("t={} {name}: golden {a}, run {b}", w["time"]));
                    }
                }
            }
            if want["samples"].as_array().map(Vec::len)
                != golden_doc["samples"].as_array().map(Vec::len)
            {
                bad.push("a different number of samples".into());
            }
            if bad.is_empty() {
                println!("golden run of {rel} matches");
            } else {
                for b in &bad {
                    println!("{b}");
                }
                std::process::exit(1);
            }
        }
        if filter.is_some() {
            println!(
                "{}",
                serde_json::to_string_pretty(&line).unwrap_or_default()
            );
        }
    }

    frame_us.sort_by(f64::total_cmp);
    let pct = |p: f64| {
        frame_us
            .get(((frame_us.len() as f64 - 1.0) * p) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let top = |m: &BTreeMap<String, usize>, n: usize| {
        let mut v: Vec<_> = m.iter().collect();
        v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        v.into_iter()
            .take(n)
            .map(|(k, c)| format!("{k} {c}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    println!("{with_scripts} vehicles with scripts run for {frames} frames");
    println!("  with a compile or load error: {load_errors}");
    println!("  with a stack underflow at run time: {underflow_vehicles}");
    println!("  with a non-finite variable: {non_finite_vehicles}");
    println!(
        "  microseconds per frame: median {:.0}, p90 {:.0}, max {:.0}",
        pct(0.5),
        pct(0.9),
        pct(1.0)
    );
    println!(
        "  unfed inputs ({} names): {}",
        unfed.len(),
        top(&unfed, 25)
    );
    println!(
        "  stubbed callbacks ({} names): {}",
        stubbed.len(),
        top(&stubbed, 25)
    );
    println!(
        "  real callbacks ({} names): {}",
        real.len(),
        top(&real, 25)
    );
    println!(
        "  unconsumed outputs ({} names): {}",
        unconsumed_count.len(),
        top(&unconsumed_count, 25)
    );
}
