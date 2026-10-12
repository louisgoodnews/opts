//! Corpus probe for the scenery runtime: loads the scripts of every `.sco` below
//! `<OMSI>/Sceneryobjects` that has a `[script]`, runs them through the game's own host
//! (`SceneryScripts::tick`) for 10 simulated seconds at noon and 10 at night with the light phase
//! cycling, and reports load problems, run-time diagnostics and cost per script file:
//!
//! `cargo run --release -p opts-bevy --example scenery_scripts -- <OMSI dir> [top N]`
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use opts_bevy::scenery_runtime::{HostValues, SceneryVariableOverrides};
use opts_bevy::{OmsiInstall, SceneryClock, SceneryScripts};
use opts_core::formats::scenery::SceneryObject;
use opts_sim::script::{load_scenery_scripts, Severity};

const DT: f32 = 1.0 / 60.0;
const FRAMES_PER_SCENARIO: usize = 600;

#[derive(Default)]
struct PerScript {
    objects: usize,
    compile_errors: usize,
    missing_files: usize,
    runtime_diags: usize,
    live: usize,
    ticks: u64,
    nanos: u128,
}

fn scos(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            scos(&p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("sco")) {
            out.push(p);
        }
    }
}

/// Runs one object through both scenarios; returns (runtime diagnostics, any variable changed,
/// ticks, nanoseconds).
fn run(scripts: &mut SceneryScripts) -> (usize, bool, u64, u128) {
    let overrides = SceneryVariableOverrides::default();
    let mut diags = 0;
    let mut snapshots = Vec::new();
    let started = Instant::now();
    for hour in [12.0, 23.0] {
        let mut clock = SceneryClock::at(2026, 6, 29, hour * 3600.0);
        for f in 0..FRAMES_PER_SCENARIO {
            let phase = ((f / 30) % 13) as f32;
            diags += scripts.tick(&HostValues::of(&clock), Some(phase), &overrides, DT);
            clock.advance(DT);
        }
        snapshots.push(scripts.with_engine(|e| e.variables()).unwrap_or_default());
    }
    let nanos = started.elapsed().as_nanos();
    let live = snapshots[0] != snapshots[1]
        || snapshots[0]
            .iter()
            .any(|(n, v)| *v != 0.0 && n != "trafficlightphase");
    (diags, live, (2 * FRAMES_PER_SCENARIO) as u64, nanos)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(root) = args.next().map(PathBuf::from) else {
        eprintln!("usage: scenery_scripts <OMSI dir> [top N]");
        std::process::exit(2);
    };
    let top: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let install = OmsiInstall::new(&root);
    let mut files = Vec::new();
    scos(&root.join("Sceneryobjects"), &mut files);
    let mut per: HashMap<String, PerScript> = HashMap::new();
    let mut messages: HashMap<String, usize> = HashMap::new();
    let (mut scripted, mut unparsable) = (0, 0);
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(sco) = SceneryObject::parse(&bytes) else {
            unparsable += 1;
            continue;
        };
        if sco.scripts.is_empty() {
            continue;
        }
        scripted += 1;
        let key = sco.scripts[0]
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or_default()
            .to_lowercase();
        let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
        let set = load_scenery_scripts(&sco, |rel| install.find_in(&dir, rel));
        let entry_key = key.clone();
        let entry = per.entry(key).or_default();
        entry.objects += 1;
        entry.compile_errors += set
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        entry.missing_files += set
            .diagnostics
            .iter()
            .filter(|d| d.message == "file not found")
            .count();
        let mut scripts = SceneryScripts::new(set.engine, &set.string_names, &[], Vec::new());
        let (diags, live, ticks, nanos) = run(&mut scripts);
        for d in scripts.diagnostics() {
            *messages
                .entry(format!("{} [{}]", d.message, entry_key))
                .or_default() += 1;
        }
        entry.runtime_diags += diags;
        entry.live += usize::from(live);
        entry.ticks += ticks;
        entry.nanos += nanos;
    }
    let mut rows: Vec<_> = per.iter().collect();
    rows.sort_by(|a, b| b.1.objects.cmp(&a.1.objects).then(a.0.cmp(b.0)));
    println!(
        "{:<36} {:>7} {:>8} {:>8} {:>8} {:>6} {:>9}",
        "script", "objects", "compile", "missing", "runtime", "live", "us/tick"
    );
    for (name, p) in rows.iter().take(top) {
        println!(
            "{:<36} {:>7} {:>8} {:>8} {:>8} {:>6} {:>9.2}",
            name,
            p.objects,
            p.compile_errors,
            p.missing_files,
            p.runtime_diags,
            p.live,
            p.nanos as f64 / p.ticks.max(1) as f64 / 1000.0
        );
    }
    let sum = |f: fn(&PerScript) -> usize| per.values().map(f).sum::<usize>();
    let objects_with = |f: fn(&PerScript) -> usize| per.values().filter(|p| f(p) > 0).count();
    let ticks: u64 = per.values().map(|p| p.ticks).sum();
    let nanos: u128 = per.values().map(|p| p.nanos).sum();
    println!(
        "\n.sco files: {} ({} unparsable), with scripts: {scripted}, distinct first scripts: {}",
        files.len(),
        unparsable,
        per.len()
    );
    println!(
        "compile errors: {} (in {} script names); missing files: {} (in {}); run-time diagnostics: {} (in {}); live objects: {}",
        sum(|p| p.compile_errors),
        objects_with(|p| p.compile_errors),
        sum(|p| p.missing_files),
        objects_with(|p| p.missing_files),
        sum(|p| p.runtime_diags),
        objects_with(|p| p.runtime_diags),
        sum(|p| p.live)
    );
    let mut top_messages: Vec<_> = messages.into_iter().collect();
    top_messages.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("most common run-time diagnostics:");
    for (m, n) in top_messages.iter().take(15) {
        println!("  {n:>5}  {m}");
    }
    println!(
        "mean cost per tick (host feed + {{frame}}): {:.2} us over {ticks} ticks",
        nanos as f64 / ticks.max(1) as f64 / 1000.0
    );
}
