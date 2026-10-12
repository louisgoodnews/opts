//! Loads the scripts of every `.bus` below `<OMSI>/Vehicles`, runs `{init}` and 60 frames and
//! reports load problems and run-time cost: `vehicle_scripts <OMSI dir>`.
use std::io::Write as _;
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
    let (mut frame_us, mut ops) = (Vec::new(), 0u64);
    let mut report = std::env::var_os("CORPUS_REPORT").and_then(|p| std::fs::File::create(p).ok());
    let mut underflow_sites = 0usize;
    let mut underflow_vehicles = std::collections::HashSet::new();
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
        // `CORPUS_REPORT=<file>` collects every load and run diagnostic for the content issue
        // registry (`scripts/content_issues.py --corpus <file>`).
        let mut report_line = |sev: &str, file: &str, msg: &str| {
            if let Some(out) = report.as_mut() {
                let bus_rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
                let file_rel = file
                    .strip_prefix(&format!("{}/", root.display()))
                    .unwrap_or(file);
                let _ = writeln!(
                    out,
                    "{bus_rel}\t{sev}\t{file_rel}\t{}",
                    msg.replace(['\t', '\n'], " ")
                );
            }
        };
        for d in diags
            .iter()
            .filter(|d| !d.message.contains("without a varlist"))
        {
            report_line(&format!("{:?}", d.severity), &d.file, &d.message);
        }
        engine.run_init();
        let t = Instant::now();
        for _ in 0..60 {
            engine.run_frame(1.0 / 60.0, false);
        }
        frame_us.push(t.elapsed().as_secs_f64() * 1e6 / 60.0);
        ops += engine.ops_executed();
        for d in engine.drain_diagnostics() {
            report_line(&format!("{:?}", d.severity), &d.file, &d.message);
            if d.message.starts_with("stack underflow") {
                underflow_sites += 1;
                underflow_vehicles.insert(f.clone());
                if std::env::var_os("SHOW_UNDERFLOW").is_some() {
                    println!(
                        "{} :: {} :: {}",
                        f.display(),
                        d.file.rsplit('/').next().unwrap_or(""),
                        d.message
                    );
                }
            }
        }
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
