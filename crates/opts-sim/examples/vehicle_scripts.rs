//! Loads the scripts of every `.bus` below `<OMSI>/Vehicles`, runs `{init}` and 60 frames and
//! reports load problems and run-time cost: `vehicle_scripts <OMSI dir>`.
use std::path::{Path, PathBuf};
use std::time::Instant;

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

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("OMSI directory"));
    let mut files = Vec::new();
    buses(&root.join("Vehicles"), &mut files);
    files.sort();
    let (mut loaded, mut with_scripts, mut missing, mut undeclared) = (0, 0, 0, 0);
    let (mut frame_us, mut ops, mut underflows) = (Vec::new(), 0u64, 0usize);
    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let Ok(bus) = BusManifest::parse_bytes(&bytes) else {
            continue;
        };
        loaded += 1;
        if bus.scripts.is_empty() {
            continue;
        }
        with_scripts += 1;
        let dir = f.parent().expect("dir").to_path_buf();
        let (mut engine, diags) = load_vehicle_scripts(&bus, |rel| {
            find_ci(&dir, rel).or_else(|| find_ci(&root, rel))
        });
        missing += diags
            .iter()
            .filter(|d| d.message == "file not found")
            .count();
        undeclared += diags
            .iter()
            .filter(|d| d.message.contains("without a varlist"))
            .count();
        engine.run_init();
        let t = Instant::now();
        for _ in 0..60 {
            engine.run_frame(1.0 / 60.0, false);
        }
        frame_us.push(t.elapsed().as_secs_f64() * 1e6 / 60.0);
        ops += engine.ops_executed();
        underflows += engine
            .drain_diagnostics()
            .iter()
            .filter(|d| d.message.contains("underflow"))
            .count();
    }
    frame_us.sort_by(f64::total_cmp);
    let pct = |p: f64| {
        frame_us
            .get(((frame_us.len() as f64 - 1.0) * p) as usize)
            .copied()
            .unwrap_or(0.0)
    };
    println!("{} .bus files, {loaded} parsed, {with_scripts} with scripts, {missing} missing files, {undeclared} with undeclared variables", files.len());
    println!(
        "frame time µs: median {:.0}, p90 {:.0}, max {:.0}; {ops} ops total",
        pct(0.5),
        pct(0.9),
        pct(1.0)
    );
    println!("stack-underflow sites: {underflows}");
}
