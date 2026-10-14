//! HUD profiles: which widgets a vehicle kind shows and where (RON data).
//!
//! The shipped profiles are `profiles/<kind>.ron`. A file of the same name in the `hud/` folder of
//! the configuration directory replaces the shipped one (third parties ship profiles as data,
//! never code). A broken file never stops the game: the problem is logged and the shipped
//! profile is used.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::widget::{widget, Slot, VehicleKind, WIDGETS};

/// One widget of a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileEntry {
    /// The widget id.
    pub id: String,
    /// Slot override; must be one the widget allows. Absent: the widget's own slot.
    #[serde(default)]
    pub slot: Option<Slot>,
    /// Order inside the slot (ascending).
    #[serde(default)]
    pub order: u16,
}

/// The widgets of one vehicle kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HudProfile {
    /// The kind it is for.
    pub kind: VehicleKind,
    /// Its widgets.
    pub widgets: Vec<ProfileEntry>,
}

/// A problem found while reading a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileProblem {
    /// The text is not valid RON for a profile.
    Syntax(String),
    /// An id that no widget has.
    UnknownWidget(String),
    /// A slot the widget does not allow.
    SlotNotAllowed(String, Slot),
    /// The same widget twice.
    Duplicate(String),
}

impl std::fmt::Display for ProfileProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(e) => write!(f, "not a HUD profile: {e}"),
            Self::UnknownWidget(id) => write!(f, "unknown widget `{id}`"),
            Self::SlotNotAllowed(id, s) => write!(f, "widget `{id}` cannot be in slot {}", s.id()),
            Self::Duplicate(id) => write!(f, "widget `{id}` listed twice"),
        }
    }
}

impl HudProfile {
    /// Parses and validates RON text. Entries with a problem are dropped and reported; syntax
    /// errors give no profile.
    pub fn parse(text: &str) -> Result<(Self, Vec<ProfileProblem>), ProfileProblem> {
        let mut profile: Self =
            ron::from_str(text).map_err(|e| ProfileProblem::Syntax(e.to_string()))?;
        let mut problems = Vec::new();
        let mut seen = Vec::<String>::new();
        profile.widgets.retain(|e| {
            let Some(w) = widget(&e.id) else {
                problems.push(ProfileProblem::UnknownWidget(e.id.clone()));
                return false;
            };
            if seen.contains(&e.id) {
                problems.push(ProfileProblem::Duplicate(e.id.clone()));
                return false;
            }
            if let Some(s) = e.slot {
                if !w.slots.contains(&s) {
                    problems.push(ProfileProblem::SlotNotAllowed(e.id.clone(), s));
                    return false;
                }
            }
            seen.push(e.id.clone());
            true
        });
        Ok((profile, problems))
    }

    /// The entries of `slot` with the widget, ordered.
    #[must_use]
    pub fn in_slot(&self, slot: Slot) -> Vec<(&ProfileEntry, &'static super::widget::HudWidget)> {
        let mut v: Vec<_> = self
            .widgets
            .iter()
            .filter_map(|e| widget(&e.id).map(|w| (e, w)))
            .filter(|(e, w)| e.slot.unwrap_or(w.slot) == slot)
            .collect();
        v.sort_by_key(|(e, _)| e.order);
        v
    }
}

impl HudProfile {
    /// The widgets of one row of the bottom-left cluster, ordered.
    #[must_use]
    pub fn in_row(
        &self,
        row: super::widget::Row,
    ) -> Vec<(&ProfileEntry, &'static super::widget::HudWidget)> {
        self.in_slot(Slot::ClusterBottomLeft)
            .into_iter()
            .filter(|(_, w)| w.binding.row() == Some(row))
            .collect()
    }
}

/// The shipped profile text of a kind.
#[must_use]
pub const fn shipped(kind: VehicleKind) -> &'static str {
    match kind {
        VehicleKind::Bus => include_str!("profiles/bus.ron"),
        VehicleKind::ArticulatedBus => include_str!("profiles/articulated-bus.ron"),
        VehicleKind::Tram => include_str!("profiles/tram.ron"),
        VehicleKind::Trolleybus => include_str!("profiles/trolleybus.ron"),
    }
}

/// All profiles, by kind.
#[derive(Resource, Debug, Clone, Default)]
pub struct HudProfiles(pub BTreeMap<VehicleKind, HudProfile>);

impl HudProfiles {
    /// The shipped profiles with the overrides of `dir` (the `hud/` folder of the configuration).
    #[must_use]
    pub fn load(dir: Option<&Path>) -> Self {
        let mut map = BTreeMap::new();
        for kind in VehicleKind::ALL {
            let user = dir.and_then(|d| read_user(&d.join(format!("{}.ron", kind.id())), kind));
            let profile = user.unwrap_or_else(|| {
                let (p, problems) = HudProfile::parse(shipped(kind))
                    .unwrap_or_else(|e| panic!("shipped profile {}: {e}", kind.id()));
                debug_assert!(problems.is_empty(), "shipped profile {}", kind.id());
                p
            });
            map.insert(kind, profile);
        }
        Self(map)
    }

    /// The profile of `kind`.
    #[must_use]
    pub fn get(&self, kind: VehicleKind) -> Option<&HudProfile> {
        self.0.get(&kind)
    }
}

fn read_user(path: &Path, kind: VehicleKind) -> Option<HudProfile> {
    let text = std::fs::read_to_string(path).ok()?;
    match HudProfile::parse(&text) {
        Ok((p, problems)) if p.kind == kind => {
            for pr in problems {
                tracing::warn!(file = %path.display(), "HUD profile: {pr}");
            }
            Some(p)
        }
        Ok(_) => {
            tracing::warn!(file = %path.display(), "HUD profile is for another vehicle kind, ignored");
            None
        }
        Err(e) => {
            tracing::warn!(file = %path.display(), "HUD profile ignored: {e}");
            None
        }
    }
}

/// The folder user profiles are read from (`<config dir>/hud`).
#[must_use]
pub fn user_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "opts", "opts").map(|d| d.config_dir().join("hud"))
}

/// Widgets that no shipped profile lists (a registry entry without a place is a mistake).
#[must_use]
pub fn unplaced(profiles: &HudProfiles) -> Vec<&'static str> {
    WIDGETS
        .iter()
        .filter(|w| {
            !profiles
                .0
                .values()
                .any(|p| p.widgets.iter().any(|e| e.id == w.id))
        })
        .map(|w| w.id)
        .collect()
}
