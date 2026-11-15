//! The catalogue of one vehicle's setvars: observed, declared and documented evidence merged,
//! sorted by group and label.

use std::collections::{BTreeMap, BTreeSet};

use crate::formats::cti::CtiSetvar;

use super::curated::{CuratedFamily, CuratedSetvar};
use super::entry::{entry, parse_value, Seen};
use super::model::SetvarEntry;

/// The options of one vehicle, sorted by group and label.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SetvarCatalogue {
    /// The offered options.
    pub entries: Vec<SetvarEntry>,
    /// Names that repaints set but the vehicle does not declare: not offered.
    pub undeclared: Vec<String>,
    /// The documented family the vehicle was matched to, if any.
    pub family: Option<String>,
}

impl SetvarCatalogue {
    /// Builds the catalogue. `declared` maps a lower-case variable name to the variable lists
    /// that declare it (empty: the declarations are unknown and every name is offered);
    /// `family` is the vendor document of the vehicle's family.
    #[must_use]
    pub fn build(
        observed: &[CtiSetvar],
        declared: &BTreeMap<String, BTreeSet<String>>,
        family: Option<&CuratedFamily>,
    ) -> Self {
        let mut seen: BTreeMap<String, Seen> = BTreeMap::new();
        for sv in observed {
            let e = seen.entry(sv.variable.trim().to_lowercase()).or_default();
            if e.name.is_empty() {
                e.name = sv.variable.trim().to_string();
            }
            e.values.insert(sv.value.trim().replace(',', "."));
            e.by_repaint
                .insert(sv.repaint.clone(), parse_value(&sv.value).unwrap_or(0.0));
        }
        let curated: BTreeMap<String, &CuratedSetvar> = family
            .into_iter()
            .flat_map(|f| f.setvars.iter())
            .map(|s| (s.name.trim().to_lowercase(), s))
            .collect();
        let mut out = Self {
            family: family.map(|f| f.id.clone()),
            ..Self::default()
        };
        let known = !declared.is_empty();
        let keys: BTreeSet<&String> = seen.keys().chain(curated.keys()).collect();
        for key in keys {
            let lists = declared.get(key.as_str());
            let is_declared = lists.is_some();
            if known && !is_declared {
                // A documented name the vehicle does not declare is not usable either.
                if let Some(s) = seen.get(key.as_str()) {
                    out.undeclared.push(s.name.clone());
                }
                continue;
            }
            let observed = seen.get(key.as_str());
            let doc = curated.get(key.as_str()).copied();
            out.entries
                .push(entry(key, observed, doc, lists, is_declared));
        }
        out.entries.sort_by(|a, b| {
            (a.group.as_deref().unwrap_or("~"), a.label.to_lowercase())
                .cmp(&(b.group.as_deref().unwrap_or("~"), b.label.to_lowercase()))
        });
        out.undeclared.sort();
        out
    }

    /// The entry of a variable (case-insensitive).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&SetvarEntry> {
        self.entries
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name.trim()))
    }
}
