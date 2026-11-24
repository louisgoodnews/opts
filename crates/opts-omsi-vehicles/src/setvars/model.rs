//! The data model of a setvar catalogue: the control kinds, the respawn and evidence facts, and
//! one option (`SetvarEntry`) with its accepted values.

use std::collections::BTreeMap;

/// Largest value of a free number (identifiers, plates) when nothing observed is larger.
pub const NUMBER_MAX: f64 = 999_999.0;

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
    pub(super) by_repaint: BTreeMap<String, f64>,
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
