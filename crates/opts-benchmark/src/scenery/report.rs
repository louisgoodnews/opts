//! Report files: `results.jsonl` (appended and flushed per object), `summary.csv`,
//! `categories.json`, `report.md`, and the comparison of two runs.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::io::{BufRead, Write as _};
use std::path::Path;

use anyhow::Result;

use super::{Record, Verdict, ViewMetrics};

/// Appends records to `results.jsonl`, flushing after each one.
#[derive(Debug)]
pub struct ResultsWriter(std::fs::File);

impl ResultsWriter {
    /// Opens (or creates) `results.jsonl` for appending.
    ///
    /// # Errors
    /// I/O failures.
    pub fn open(path: &Path) -> Result<Self> {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self(f))
    }

    /// Appends one record.
    ///
    /// # Errors
    /// I/O or JSON failures.
    pub fn append(&mut self, r: &Record) -> Result<()> {
        writeln!(self.0, "{}", serde_json::to_string(r)?)?;
        self.0.flush()?;
        Ok(())
    }
}

/// Reads `results.jsonl`; a truncated last line (crash while writing) is skipped.
#[must_use]
pub fn read_results(path: &Path) -> Vec<Record> {
    let Ok(f) = std::fs::File::open(path) else {
        return Vec::new();
    };
    std::io::BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

/// The last record per key (a resumed run may have retried an object).
#[must_use]
pub fn latest(records: Vec<Record>) -> Vec<Record> {
    let mut by_key: BTreeMap<String, Record> = BTreeMap::new();
    for r in records {
        by_key.insert(r.key.clone(), r);
    }
    by_key.into_values().collect()
}

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

/// Category → count and the first 20 object paths.
#[must_use]
pub fn categories(records: &[Record]) -> BTreeMap<String, serde_json::Value> {
    let mut map: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    for r in records {
        for c in &r.categories {
            let e = map.entry(c.clone()).or_default();
            e.0 += 1;
            if e.1.len() < 20 {
                e.1.push(r.sco.clone());
            }
        }
    }
    map.into_iter()
        .map(|(k, (n, ex))| (k, serde_json::json!({"count": n, "examples": ex})))
        .collect()
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

fn markdown(records: &[Record]) -> String {
    let mut s = String::from("# Scenery benchmark report\n\n");
    let mut verdicts: BTreeMap<String, usize> = BTreeMap::new();
    for r in records {
        *verdicts
            .entry(format!("{:?}", r.verdict).to_lowercase())
            .or_default() += 1;
    }
    let _ = writeln!(
        s,
        "Objects: {}\n\n| Verdict | Count |\n| :--- | ---: |",
        records.len()
    );
    for (v, n) in &verdicts {
        let _ = writeln!(s, "| {v} | {n} |");
    }
    let _ = writeln!(
        s,
        "\n| Category | Count | Example |\n| :--- | ---: | :--- |"
    );
    let mut cats: Vec<_> = categories(records).into_iter().collect();
    cats.sort_by_key(|(_, v)| std::cmp::Reverse(v["count"].as_u64().unwrap_or(0)));
    for (c, v) in cats {
        let ex = v["examples"][0].as_str().unwrap_or("");
        let _ = writeln!(s, "| {c} | {} | `{ex}` |", v["count"]);
    }
    let mut by_class: BTreeMap<String, HashMap<Verdict, usize>> = BTreeMap::new();
    for r in records {
        for c in &r.class {
            *by_class
                .entry(c.clone())
                .or_default()
                .entry(r.verdict)
                .or_default() += 1;
        }
    }
    let _ = writeln!(
        s,
        "\n| Class | pass | fail | unknown |\n| :--- | ---: | ---: | ---: |"
    );
    for (c, m) in &by_class {
        let g = |v| m.get(&v).copied().unwrap_or(0);
        let _ = writeln!(
            s,
            "| {c} | {} | {} | {} |",
            g(Verdict::Pass),
            g(Verdict::Fail),
            g(Verdict::Unknown)
        );
    }
    let mut totals: Vec<u64> = records.iter().map(|r| r.timing_ms.total).collect();
    totals.sort_unstable();
    let _ = writeln!(
        s,
        "\nTiming per object (ms): median {}, p95 {}, max {}\n\nSlowest 20:\n",
        percentile(&totals, 0.5),
        percentile(&totals, 0.95),
        totals.last().copied().unwrap_or(0)
    );
    let mut slow: Vec<&Record> = records.iter().collect();
    slow.sort_by_key(|r| std::cmp::Reverse(r.timing_ms.total));
    for r in slow.iter().take(20) {
        let _ = writeln!(s, "- {} ms `{}`", r.timing_ms.total, r.sco);
    }
    s
}

/// Lists the objects whose verdict or categories differ between two runs.
#[must_use]
pub fn compare(a: &[Record], b: &[Record]) -> String {
    let index: HashMap<&str, &Record> = a.iter().map(|r| (r.key.as_str(), r)).collect();
    let mut s = String::new();
    let (mut same, mut changed) = (0usize, 0usize);
    for r in b {
        let Some(old) = index.get(r.key.as_str()) else {
            continue;
        };
        if old.verdict == r.verdict && old.categories == r.categories {
            same += 1;
            continue;
        }
        changed += 1;
        let _ = writeln!(
            s,
            "{}: {:?} {:?} -> {:?} {:?}",
            r.sco, old.verdict, old.categories, r.verdict, r.categories
        );
    }
    let _ = writeln!(s, "{changed} changed, {same} unchanged");
    s
}

#[cfg(test)]
mod tests;
