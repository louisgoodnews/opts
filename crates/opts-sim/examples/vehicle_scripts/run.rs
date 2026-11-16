//! One `.bus` run: load its scripts, run 60 frames and record the diagnostics.

use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use opts_core::formats::vehicles::BusManifest;
use opts_sim::script::load_vehicle_scripts;

use super::discovery::find_ci;
use super::Tally;

/// Writes one tab-separated diagnostic line to the `CORPUS_REPORT` file, when there is one.
fn report_line(
    report: &mut Option<std::fs::File>,
    root: &Path,
    bus: &Path,
    (sev, file, msg): (&str, &str, &str),
) {
    if let Some(out) = report.as_mut() {
        let bus_rel = bus.strip_prefix(root).unwrap_or(bus).display().to_string();
        let file_rel = file
            .strip_prefix(&format!("{}/", root.display()))
            .unwrap_or(file);
        let _ = writeln!(
            out,
            "{bus_rel}\t{sev}\t{file_rel}\t{}",
            msg.replace(['\t', '\n'], " ")
        );
    }
}

/// Loads the scripts of one `.bus`, runs 60 frames and records what happened.
pub(crate) fn run_vehicle(
    f: &Path,
    root: &Path,
    report: &mut Option<std::fs::File>,
    tally: &mut Tally,
) {
    let Ok(bytes) = std::fs::read(f) else {
        return;
    };
    let Ok(bus) = BusManifest::parse_bytes(&bytes) else {
        return;
    };
    tally.loaded += 1;
    if bus.scripts.is_empty() {
        return;
    }
    tally.with_scripts += 1;
    let dir = f.parent().expect("dir").to_path_buf();
    let (mut engine, diags) = load_vehicle_scripts(&bus, |rel| {
        find_ci(&dir, rel).or_else(|| find_ci(root, rel))
    });
    tally.missing += diags
        .iter()
        .filter(|d| d.message == "file not found")
        .count();
    tally.undeclared += diags
        .iter()
        .filter(|d| d.message.contains("without a varlist"))
        .count();
    // `CORPUS_REPORT=<file>` collects every load and run diagnostic for the content issue
    // registry (`scripts/content_issues.py --corpus <file>`).
    for d in diags
        .iter()
        .filter(|d| !d.message.contains("without a varlist"))
    {
        let sev = format!("{:?}", d.severity);
        report_line(report, root, f, (&sev, &d.file, &d.message));
    }
    engine.run_init();
    let t = Instant::now();
    for _ in 0..60 {
        engine.run_frame(1.0 / 60.0, false);
    }
    tally.frame_us.push(t.elapsed().as_secs_f64() * 1e6 / 60.0);
    tally.ops += engine.ops_executed();
    for d in engine.drain_diagnostics() {
        let sev = format!("{:?}", d.severity);
        report_line(report, root, f, (&sev, &d.file, &d.message));
        if d.message.starts_with("stack underflow") {
            tally.underflow_sites += 1;
            tally.underflow_vehicles.insert(f.to_path_buf());
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
