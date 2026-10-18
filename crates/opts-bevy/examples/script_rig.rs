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
    while let Some(a) = args.next() {
        match a.as_str() {
            "--vehicle" => filter = args.next().map(|s| s.to_lowercase()),
            "--frames" => frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(720),
            "--json" => json = args.next().map(PathBuf::from),
            "--csv" => csv = args.next().map(PathBuf::from),
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
