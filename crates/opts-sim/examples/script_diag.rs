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
    // `SCRIPT_TRIGGERS=a,b` raise triggers, `SCRIPT_SET=x=1,y=2` set variables before the frames,
    // `SCRIPT_WATCH=n,m` prints those variables every 30 frames.
    // Triggers are `name` (before the first frame) or `name@frame`.
    let triggers: Vec<(String, usize)> = std::env::var("SCRIPT_TRIGGERS")
        .unwrap_or_default()
        .split(',')
        .filter(|t| !t.is_empty())
        .map(|t| match t.split_once('@') {
            Some((n, f)) => (n.to_owned(), f.parse().unwrap_or(0)),
            None => (t.to_owned(), 0),
        })
        .collect();
    for kv in std::env::var("SCRIPT_SET")
        .unwrap_or_default()
        .split(',')
        .filter(|t| !t.is_empty())
    {
        if let Some((k, v)) = kv.split_once('=') {
            engine.set(k, v.parse().unwrap_or(0.0));
        }
    }
    let frames: usize = std::env::var("SCRIPT_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let watch: Vec<String> = std::env::var("SCRIPT_WATCH")
        .unwrap_or_default()
        .split(',')
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect();
    for f in 0..frames {
        for (t, at) in &triggers {
            if *at == f {
                // `name` raises a trigger, `M:name` calls a macro.
                match t.strip_prefix("M:") {
                    Some(m) => println!("frame {f}: macro {m}: {}", engine.call_macro(m)),
                    None => println!("frame {f}: trigger {t}: {}", engine.trigger(t)),
                }
            }
        }
        engine.run_frame(1.0 / 60.0, false);
        if !watch.is_empty()
            && f % std::env::var("SCRIPT_EVERY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30usize)
                == 0
        {
            let vals: Vec<String> = watch
                .iter()
                .map(|w| format!("{w}={:.2}", engine.get(w)))
                .collect();
            println!("frame {f}: {}", vals.join(" "));
        }
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
