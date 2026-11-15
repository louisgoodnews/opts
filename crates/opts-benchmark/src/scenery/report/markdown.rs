//! The Markdown report: verdicts, categories, classes, timing and the slowest objects.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

use super::super::{Record, Verdict};
use super::category::categories;

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

pub(super) fn markdown(records: &[Record]) -> String {
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
