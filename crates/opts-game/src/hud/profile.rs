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

mod problem;
mod store;

pub use problem::ProfileProblem;
pub use store::{user_dir, unplaced, HudProfiles};

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
