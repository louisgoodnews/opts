//! Loads the scripts of every `.bus` below `<OMSI>/Vehicles`, runs `{init}` and 60 frames and
//! reports load problems and run-time cost: `vehicle_scripts <OMSI dir>`.
#[path = "vehicle_scripts/discovery.rs"]
mod discovery;
#[path = "vehicle_scripts/run.rs"]
mod run;

use std::path::PathBuf;

use discovery::buses;
use run::run_vehicle;

/// What the run over the corpus counts.
#[derive(Default)]
struct Tally {
    loaded: usize,
    with_scripts: usize,
    missing: usize,
    undeclared: usize,
    frame_us: Vec<f64>,
    ops: u64,
    underflow_sites: usize,
    underflow_vehicles: std::collections::HashSet<PathBuf>,
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI directory"));
    let mut files = Vec::new();
    buses(&root.join("Vehicles"), &mut files);
    files.sort();
    let mut report = std::env::var_os("CORPUS_REPORT").and_then(|p| std::fs::File::create(p).ok());
    let mut tally = Tally::default();
    for f in &files {
        run_vehicle(f, &root, &mut report, &mut tally);
    }
    let mut frame_us = std::mem::take(&mut tally.frame_us);
    frame_us.sort_by(f64::total_cmp);
    let pct = |p: f64| {
        frame_us
            .get(((frame_us.len() as f64 - 1.0) * p) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    let Tally {
        loaded,
        with_scripts,
        missing,
        undeclared,
        ops,
        underflow_sites,
        underflow_vehicles,
        ..
    } = &tally;
    println!("{} .bus files, {loaded} parsed, {with_scripts} with scripts, {missing} missing files, {undeclared} with undeclared variables", files.len());
    println!(
        "stack underflow: {underflow_sites} sites in {} vehicles",
        underflow_vehicles.len()
    );
    println!(
        "frame time µs: median {:.0}, p90 {:.0}, max {:.0}; {ops} ops total",
        pct(0.5),
        pct(0.9),
        pct(1.0)
    );
}
