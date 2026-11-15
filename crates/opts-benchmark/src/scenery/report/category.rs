//! Category counts with the first object paths of each category.

use std::collections::BTreeMap;

use super::super::Record;

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
