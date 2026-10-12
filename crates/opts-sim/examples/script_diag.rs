//! Runs one vehicle's scripts and prints the run-time diagnostics:
//! `script_diag <OMSI dir> <.bus file> [block name to disassemble]`.
use std::path::{Path, PathBuf};

use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::load_vehicle_scripts;

fn find_ci(base: &Path, rel: &str) -> Option<PathBuf> {
    let mut cur = base.to_path_buf();
    for comp in rel
        .split(['\\', '/'])
        .filter(|c| !c.is_empty() && *c != ".")
    {
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

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("OMSI directory"));
    let bus_path = PathBuf::from(args.next().expect(".bus file"));
    let block = args.next();
    let bus = BusManifest::parse_bytes(&std::fs::read(&bus_path).expect("read")).expect("parse");
    let dir = bus_path.parent().expect("dir").to_path_buf();
    let (mut engine, diags) = load_vehicle_scripts(&bus, |rel| {
        find_ci(&dir, rel).or_else(|| find_ci(&root, rel))
    });
    for d in &diags {
        println!("load: {:?} {} {}", d.severity, d.file, d.message);
    }
    engine.run_init();
    for _ in 0..30 {
        engine.run_frame(1.0 / 60.0, false);
    }
    for d in engine.drain_diagnostics() {
        println!("run: {:?} {} {}", d.severity, d.file, d.message);
    }
    for (n, v) in engine.string_variables() {
        if std::env::var_os("SHOW_STRINGS").is_some() {
            println!("str {n} = {v:?}");
        }
    }
    if let Some(b) = block {
        for l in engine.disassemble(&b) {
            println!("{l}");
        }
    }
}
