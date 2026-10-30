//! The setvars of one vehicle (M-0078): which equipment options it offers, of what kind, with
//! which values, and what they are called. See `docs/src/formats/text/vehicles/setvars.md`.
//!
//! The catalogue merges three kinds of evidence:
//! 1. **Observed:** the `[setvar]` lines of the vehicle's repaints (`.cti`): the name, the values
//!    repaints give it (a lower bound of the range) and each repaint's own value.
//! 2. **Declared:** the vehicle's `[varnamelist]` files: a name is only offered when the vehicle
//!    declares it (scripts read it); an unknown declaration (no lists given) offers everything.
//! 3. **Documented:** the vendor's setvar document, restated in `data/setvar_families.json`
//!    (names, short meanings, value ranges, groups), matched through the vehicle folder.
//!
//! Pure data: no file access; callers read the files and pass the parsed pieces.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::formats::cti::CtiSetvar;

/// Largest value of a free number (identifiers, plates) when nothing observed is larger.
pub const NUMBER_MAX: f64 = 999_999.0;
/// More distinct integer values than this make an option a free number, not a choice.
const MAX_CHOICES: usize = 100;
/// Longest meaning text that is used as the label itself.
const LABEL_MAX_CHARS: usize = 36;

/// What kind of control an option needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetvarKind {
    /// Off and on (values within 0 and 1).
    Switch,
    /// One of a few integers (counted from 0 or 1, or codes with gaps).
    Choice,
    /// A free whole number (a vehicle number, a plate part).
    Number,
    /// A decimal value (door speeds).
    Fraction,
}

/// Whether a change of an option reaches a vehicle that already exists (Q-SV02). Only documented
/// or tested facts are `Required` or `Live`; everything else stays `Unknown`, never a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Respawn {
    /// The vendor documents that the vehicle must be built again after a change.
    Required,
    /// A live test showed that the running vehicle reacts.
    Live,
    /// Nothing is known about the option.
    #[default]
    Unknown,
}

/// Which evidence an entry rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Evidence {
    /// Some repaint of the vehicle sets it.
    pub observed: bool,
    /// One of the vehicle's variable lists declares it.
    pub declared: bool,
    /// The vendor's document lists it.
    pub documented: bool,
}

/// One option of a vehicle.
#[derive(Debug, Clone, PartialEq)]
pub struct SetvarEntry {
    /// The script variable, spelled as the repaints (or the document) write it.
    pub name: String,
    /// Control kind.
    pub kind: SetvarKind,
    /// Smallest accepted value.
    pub min: f64,
    /// Largest accepted value.
    pub max: f64,
    /// The accepted integers of a [`SetvarKind::Choice`] (sorted); empty for the other kinds.
    pub values: Vec<i64>,
    /// What the option is called.
    pub label: String,
    /// More about it; empty when there is nothing to say.
    pub description: String,
    /// Vendor group (`display`, `doors`, ...), if documented.
    pub group: Option<String>,
    /// The variable lists that declare it.
    pub declared_in: Vec<String>,
    /// What the entry rests on.
    pub evidence: Evidence,
    /// The documented range was narrower than the values repaints set (a picture was probably
    /// misread), so the range is the join of both.
    pub range_unverified: bool,
    /// Whether a change needs the vehicle to be built again (documented or tested only).
    pub respawn: Respawn,
    /// The value each repaint gives the option.
    by_repaint: BTreeMap<String, f64>,
}

impl SetvarEntry {
    /// The value `repaint` gives the option; `0` (not set) when it gives none.
    #[must_use]
    pub fn default_for(&self, repaint: &str) -> f64 {
        self.by_repaint.get(repaint).copied().unwrap_or(0.0)
    }

    /// Whether `value` is accepted.
    #[must_use]
    pub fn accepts(&self, value: f64) -> bool {
        match self.kind {
            SetvarKind::Choice => self.values.iter().any(|v| (*v as f64 - value).abs() < 1e-9),
            SetvarKind::Switch => value == 0.0 || value == 1.0,
            SetvarKind::Number => value.fract() == 0.0 && (self.min..=self.max).contains(&value),
            SetvarKind::Fraction => (self.min..=self.max).contains(&value),
        }
    }
}

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

#[derive(Default)]
struct Seen {
    name: String,
    values: BTreeSet<String>,
    by_repaint: BTreeMap<String, f64>,
}

fn parse_value(text: &str) -> Option<f64> {
    text.trim().replace(',', ".").parse().ok()
}

fn entry(
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

/// The label and the description: a short documented meaning is the label itself, a long one the
/// description under the readable name.
fn labels(name: &str, doc: Option<&CuratedSetvar>) -> (String, String) {
    let meaning = doc.map(|d| d.meaning.trim()).unwrap_or_default();
    if meaning.is_empty() {
        return (humanise(name), String::new());
    }
    let short = meaning.chars().count() <= LABEL_MAX_CHARS && !meaning.contains([';', '(']);
    if short {
        (capitalise(meaning), String::new())
    } else {
        (humanise(name), capitalise(meaning))
    }
}

fn capitalise(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// The option's own name made readable (`vis_front_light` becomes `Vis front light`).
#[must_use]
pub fn humanise(name: &str) -> String {
    capitalise(name.replace(['_', '-'], " ").trim())
}

/// A documented range or list of whole numbers: `0|1`, `0-3`, `0,1,10`; anything else (`list`,
/// `LINE 0|1 ; CITY 0-2`) is not a plain range and gives `None`.
#[must_use]
pub fn parse_range(text: &str) -> Option<BTreeSet<i64>> {
    let t = text.trim();
    if let Some((a, b)) = t.split_once('-') {
        let (a, b): (i64, i64) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
        return (a <= b && b - a <= 1000).then(|| (a..=b).collect());
    }
    let set: Option<BTreeSet<i64>> = t.split(['|', ',']).map(|p| p.trim().parse().ok()).collect();
    set.filter(|s| !s.is_empty())
}

/// One documented setvar of a vendor document.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CuratedSetvar {
    /// The script variable.
    pub name: String,
    /// The documented values as text (`0|1`, `0-3`, `list`, ...).
    #[serde(default)]
    pub values: String,
    /// A short meaning in our own words.
    #[serde(default)]
    pub meaning: String,
    /// Vendor group.
    #[serde(default)]
    pub group: String,
    /// `true`: the vendor says the vehicle must be built again after a change; `false`: a live
    /// test showed the running vehicle reacts; absent: not known (Q-SV02).
    #[serde(default)]
    pub respawn: Option<bool>,
}

/// The setvars one vendor documents for a family of vehicles.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CuratedFamily {
    /// Stable id (`citybus-o530-kajosoft`).
    pub id: String,
    /// The family's name.
    pub title: String,
    /// Folders below `Vehicles` that belong to the family.
    #[serde(default)]
    pub folders: Vec<String>,
    /// The documented setvars.
    pub setvars: Vec<CuratedSetvar>,
}

#[derive(Deserialize)]
struct CuratedFile {
    families: Vec<CuratedFamily>,
}

/// The documented families (parsed once from the data file shipped with the program).
#[must_use]
pub fn curated_families() -> &'static [CuratedFamily] {
    static FAMILIES: OnceLock<Vec<CuratedFamily>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        serde_json::from_str::<CuratedFile>(include_str!("../data/setvar_families.json"))
            .map(|f| f.families)
            .unwrap_or_default()
    })
}

/// The documented family of a vehicle folder below `Vehicles` (case-insensitive).
#[must_use]
pub fn family_of_folder(folder: &str) -> Option<&'static CuratedFamily> {
    curated_families().iter().find(|f| {
        f.folders
            .iter()
            .any(|x| x.trim().eq_ignore_ascii_case(folder.trim()))
    })
}

#[cfg(test)]
mod tests;
