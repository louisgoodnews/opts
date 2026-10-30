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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::inputs::{feed, FeedPlan, OUTPUT_TABLE};
use opts_sim::script::{load_vehicle_scripts, Severity};

const DT: f32 = 1.0 / 60.0;

mod args;
use args::Args;

mod modes;
use modes::{physics_mode, route_mode};

mod physics;
use physics::{physics_diff, physics_report, GOLDEN_TIMES, GOLDEN_VARS};

mod phases;
use phases::Startup;

mod route;
use route::{buses, depot_for, find_ci, route_report, seed_of, unconsumed, ENGINE_SET};

mod summary;
use summary::{print_summary, write_or_check_golden, SummaryCounts};

mod vehicle;
use vehicle::{report_vehicle, run_vehicle, Totals, VehicleInfo};

fn main() {
    let args = Args::parse();
    let mut files = Vec::new();
    buses(&args.root.join("Vehicles"), &mut files);
    files.sort();
    let mut out = args.json.as_ref().and_then(|p| std::fs::File::create(p).ok());
    if let Some(dir) = &args.csv {
        let _ = std::fs::create_dir_all(dir);
    }

    let mut totals = Totals::default();
    for f in &files {
        let rel = f
            .strip_prefix(&args.root)
            .unwrap_or(f)
            .display()
            .to_string();
        if args
            .filter
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
        if totals.with_scripts >= args.limit {
            break;
        }
        totals.with_scripts += 1;
        let dir = f.parent().expect("dir").to_path_buf();
        let (mut engine, load) = load_vehicle_scripts(&bus, |r| {
            find_ci(&dir, r).or_else(|| find_ci(&args.root, r))
        });
        let load_errors_here = load
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        totals.load_errors += usize::from(load_errors_here > 0);
        if args.route_code.is_some() || args.timetable.is_some() {
            route_mode(&args, &bus, &dir, &rel);
            continue;
        }
        if args.physics {
            physics_mode(&args, f, &bus, &dir, &rel);
            continue;
        }
        let run = run_vehicle(&args, f, &rel, &mut engine);
        let info = VehicleInfo {
            rel: &rel,
            scripts: bus.scripts.len(),
            load_errors: load_errors_here,
            load_warnings: load
                .iter()
                .filter(|d| d.severity == Severity::Warning)
                .count(),
        };
        report_vehicle(&args, &mut totals, &info, &engine, run, &mut out);
    }

    print_summary(
        SummaryCounts {
            with_scripts: totals.with_scripts,
            frames: args.frames,
            load_errors: totals.load_errors,
            underflow_vehicles: totals.underflow_vehicles,
            non_finite_vehicles: totals.non_finite_vehicles,
            unfed: &totals.unfed,
            stubbed: &totals.stubbed,
            real: &totals.real,
            unconsumed_count: &totals.unconsumed_count,
        },
        &mut totals.frame_us,
    );
}
