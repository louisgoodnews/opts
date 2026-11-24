//! One option of a vehicle, merged from its repaints (observed), its variable lists (declared)
//! and the vendor document (documented): the kind, the accepted range, the default and the label.

use std::collections::{BTreeMap, BTreeSet};

use super::curated::{parse_range, CuratedSetvar};
use super::labels::labels;
use super::model::{Evidence, Respawn, SetvarEntry, SetvarKind, NUMBER_MAX};

/// More distinct integer values than this make an option a free number, not a choice.
const MAX_CHOICES: usize = 100;

#[derive(Default)]
pub(super) struct Seen {
    pub(super) name: String,
    pub(super) values: BTreeSet<String>,
    pub(super) by_repaint: BTreeMap<String, f64>,
}

pub(super) fn parse_value(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok()
}

pub(super) fn entry(
    key: &str,
    observed: Option<&Seen>,
    doc: Option<&CuratedSetvar>,
    lists: Option<&BTreeSet<String>>,
    declared: bool,
) -> SetvarEntry {
    let name = observed
        .map(|s| s.name.clone())
        .or_else(|| doc.map(|d| d.name.trim().to_string()))
        .unwrap_or_else(|| key.to_string());
    let seen_values: Vec<f64> = observed
        .into_iter()
        .flat_map(|s| s.values.iter())
        .filter_map(|v| parse_value(v))
        .collect();
    let documented = doc.and_then(|d| parse_range(&d.values));
    let all_whole = seen_values.iter().all(|v| v.fract() == 0.0);
    let mut ints: BTreeSet<i64> = seen_values.iter().map(|v| *v as i64).collect();
    let seen_ints = ints.clone();
    if all_whole {
        if let Some(range) = &documented {
            ints.extend(range);
        }
    }
    let (kind, min, max, values) = if !all_whole {
        let lo = seen_values.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = seen_values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        (SetvarKind::Fraction, lo.min(0.0), hi.max(0.0), Vec::new())
    } else if ints.is_empty() {
        (SetvarKind::Switch, 0.0, 1.0, Vec::new())
    } else if ints.len() > MAX_CHOICES || ints.iter().any(|v| *v > MAX_CHOICES as i64) {
        let hi = ints.iter().next_back().map_or(0, |v| *v) as f64;
        (SetvarKind::Number, 0.0, hi.max(NUMBER_MAX), Vec::new())
    } else if ints.iter().all(|v| *v == 0 || *v == 1) {
        (SetvarKind::Switch, 0.0, 1.0, Vec::new())
    } else {
        let lo = ints.iter().next().map_or(0, |v| *v) as f64;
        let hi = ints.iter().next_back().map_or(0, |v| *v) as f64;
        (SetvarKind::Choice, lo, hi, ints.iter().copied().collect())
    };
    let range_unverified = documented
        .as_ref()
        .is_some_and(|r| !seen_ints.is_empty() && !seen_ints.is_subset(r));
    let (label, description) = labels(&name, doc);
    SetvarEntry {
        name,
        kind,
        min,
        max,
        values,
        label,
        description,
        group: doc
            .map(|d| d.group.trim().to_string())
            .filter(|g| !g.is_empty()),
        declared_in: lists
            .map(|l| l.iter().cloned().collect())
            .unwrap_or_default(),
        evidence: Evidence {
            observed: observed.is_some(),
            declared,
            documented: doc.is_some(),
        },
        range_unverified,
        respawn: match doc.and_then(|d| d.respawn) {
            Some(true) => Respawn::Required,
            Some(false) => Respawn::Live,
            None => Respawn::Unknown,
        },
        by_repaint: observed.map(|s| s.by_repaint.clone()).unwrap_or_default(),
    }
}
