//! Comparison of two runs: the objects whose verdict or categories differ.

use std::collections::HashMap;
use std::fmt::Write as _;

use super::super::Record;

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
