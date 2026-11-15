//! The summary files of a run: `summary.csv`, `categories.json` and `report.md`.

use std::fmt::Write as _;
use std::path::Path;

use anyhow::Result;

use super::super::{Record, ViewMetrics};
use super::category::categories;
use super::markdown::markdown;

fn csv(field: &str) -> String {
    if field.contains([',', '"', '\n']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn opt(v: Option<f32>) -> String {
    v.map(|x| format!("{x:.4}")).unwrap_or_default()
}

fn view_cols(v: Option<&ViewMetrics>) -> Vec<String> {
    let Some(v) = v else {
        return vec![String::new(); 10];
    };
    vec![
        v.px_rendered.to_string(),
        v.px_expected.to_string(),
        opt(v.precision),
        opt(v.recall),
        opt(v.iou),
        opt(v.bbox_overshoot),
        opt(v.centroid_offset),
        opt(v.size_ratio.map(|s| s[0].max(s[1]))),
        format!("{:.4}", v.alpha_share),
        format!("{:.4}", v.sentinel_share),
    ]
}

/// Writes `summary.csv`, `categories.json` and `report.md` for the records.
///
/// # Errors
/// I/O failures.
pub fn write_summaries(run_dir: &Path, records: &[Record]) -> Result<()> {
    let mut out =
        String::from("sco,key,tier,class,verdict,categories,unknown_reason,unsupported,script");
    for v in ["v1", "v2"] {
        for c in [
            "rendered",
            "expected",
            "precision",
            "recall",
            "iou",
            "overshoot",
            "centroid",
            "size",
            "alpha",
            "sentinel",
        ] {
            let _ = write!(out, ",{v}_{c}");
        }
    }
    out.push_str(",best_rotation,events,total_ms\n");
    for r in records {
        let mut cols = vec![
            r.sco.clone(),
            r.key.clone(),
            r.tier.clone(),
            r.class.join(" "),
            format!("{:?}", r.verdict).to_lowercase(),
            r.categories.join(" "),
            r.unknown_reason.clone().unwrap_or_default(),
            r.tags.unsupported.join(" "),
            r.tags.script.clone().unwrap_or_default(),
        ];
        cols.extend(view_cols(r.views.get("v1")));
        cols.extend(view_cols(r.views.get("v2")));
        cols.push(
            r.views
                .get("v2")
                .and_then(|v| v.best_rotation.as_ref())
                .map(|(n, i)| format!("{n}:{i:.3}"))
                .unwrap_or_default(),
        );
        cols.push(r.diagnostics.events.len().to_string());
        cols.push(r.timing_ms.total.to_string());
        let line: Vec<String> = cols.iter().map(|c| csv(c)).collect();
        let _ = writeln!(out, "{}", line.join(","));
    }
    std::fs::write(run_dir.join("summary.csv"), out)?;
    std::fs::write(
        run_dir.join("categories.json"),
        serde_json::to_string_pretty(&categories(records))?,
    )?;
    std::fs::write(run_dir.join("report.md"), markdown(records))?;
    Ok(())
}
