//! The golden samples and the CSV rows of one vehicle's run.

use std::collections::BTreeMap;
use std::io::Write as _;

use opts_sim::script::ScriptEngine;

use super::super::*;

/// The golden sample (`GOLDEN_VARS`) when `t` is one of the `GOLDEN_TIMES`.
pub(super) fn golden_sample(engine: &ScriptEngine, t: f32) -> Option<serde_json::Value> {
    if !GOLDEN_TIMES.iter().any(|g| (t - g).abs() < DT * 0.5) {
        return None;
    }
    let values: serde_json::Map<String, serde_json::Value> = GOLDEN_VARS
        .iter()
        .map(|n| ((*n).to_owned(), serde_json::json!(engine.get(n))))
        .collect();
    Some(
        serde_json::json!({ "time": GOLDEN_TIMES.iter().copied().find(|g| (t - g).abs() < DT * 0.5), "values": values }),
    )
}

/// One CSV row of every variable; the first frame writes the header and fixes the column order.
pub(super) fn write_csv_row(
    w: &mut std::fs::File,
    names: &mut Vec<String>,
    frame: usize,
    t: f32,
    vars: &[(String, f32)],
) {
    if frame == 0 {
        *names = vars.iter().map(|(n, _)| n.clone()).collect();
        names.sort();
        let _ = writeln!(w, "time,{}", names.join(","));
    }
    let map: BTreeMap<&str, f32> = vars.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    let row: Vec<String> = names
        .iter()
        .map(|n| format!("{}", map.get(n.as_str()).copied().unwrap_or(0.0)))
        .collect();
    let _ = writeln!(w, "{t:.4},{}", row.join(","));
}
