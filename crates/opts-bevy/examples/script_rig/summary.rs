//! Golden comparison and summary printing for the script rig example.

use std::collections::BTreeMap;
use std::path::Path;

use super::physics::GOLDEN_VARS;

pub(super) fn write_or_check_golden(
    rel: &str,
    samples: &[serde_json::Value],
    golden: Option<&Path>,
    check: Option<&Path>,
) {
    let golden_doc =
        serde_json::json!({ "vehicle": rel, "frames_per_second": 60, "samples": samples });
    if let Some(path) = golden {
        std::fs::write(
            path,
            serde_json::to_string_pretty(&golden_doc).unwrap_or_default(),
        )
        .expect("write the golden file");
        println!("golden run of {rel} written to {}", path.display());
    }
    if let Some(path) = check {
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
}

pub(super) struct SummaryCounts<'a> {
    pub with_scripts: usize,
    pub frames: usize,
    pub load_errors: usize,
    pub underflow_vehicles: usize,
    pub non_finite_vehicles: usize,
    pub unfed: &'a BTreeMap<String, usize>,
    pub stubbed: &'a BTreeMap<String, usize>,
    pub real: &'a BTreeMap<String, usize>,
    pub unconsumed_count: &'a BTreeMap<String, usize>,
}

pub(super) fn print_summary(counts: SummaryCounts<'_>, frame_us: &mut [f64]) {
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
    println!(
        "{} vehicles with scripts run for {} frames",
        counts.with_scripts, counts.frames
    );
    println!("  with a compile or load error: {}", counts.load_errors);
    println!(
        "  with a stack underflow at run time: {}",
        counts.underflow_vehicles
    );
    println!(
        "  with a non-finite variable: {}",
        counts.non_finite_vehicles
    );
    println!(
        "  microseconds per frame: median {:.0}, p90 {:.0}, max {:.0}",
        pct(0.5),
        pct(0.9),
        pct(1.0)
    );
    println!(
        "  unfed inputs ({} names): {}",
        counts.unfed.len(),
        top(counts.unfed, 25)
    );
    println!(
        "  stubbed callbacks ({} names): {}",
        counts.stubbed.len(),
        top(counts.stubbed, 25)
    );
    println!(
        "  real callbacks ({} names): {}",
        counts.real.len(),
        top(counts.real, 25)
    );
    println!(
        "  unconsumed outputs ({} names): {}",
        counts.unconsumed_count.len(),
        top(counts.unconsumed_count, 25)
    );
}
