//! Equipment options of a vehicle: its setvars (M-0078), from the [`SetvarCatalogue`] that merges
//! the `[setvar]` lines of its repaints, the variables it declares and the vendor's documentation,
//! with plain-language labels. See `docs/src/formats/text/vehicles/setvars.md`.

use std::collections::HashMap;

use opts_core::formats::cti::CtiSetvar;
use opts_core::setvars::{family_of_folder, SetvarCatalogue, SetvarKind};
use opts_gui::l10n::Localization;

use super::repaints::{vehicle_repaints, VehicleRepaints};
use super::types::VehicleItem;

/// A configurable equipment option of a vehicle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetvarItem {
    /// The script variable.
    pub name: String,
    /// What the option is called for the user.
    pub label: String,
    /// One more line, empty when there is nothing to say.
    pub description: String,
    /// Current value.
    pub value: i32,
    /// The value the chosen repaint gives it.
    pub default_value: i32,
    /// Smallest value.
    pub min_val: i32,
    /// Largest value.
    pub max_val: i32,
    /// Which control fits: a switch, a choice among `values`, or a free number.
    pub kind: SetvarKind,
    /// The accepted values of a choice (sorted; the current value is always in it).
    pub values: Vec<i32>,
    /// Vendor group (`display`, `doors`, ...), if documented.
    pub group: Option<String>,
    /// The documented range was narrower than what repaints set (shown as unverified).
    pub unverified: bool,
}

impl SetvarItem {
    /// The value after `delta` steps from `current`: a switch toggles, a choice moves through its
    /// values and wraps, a number moves by one and stays inside its range.
    #[must_use]
    pub fn step(&self, current: i32, delta: i32) -> i32 {
        match self.kind {
            SetvarKind::Switch | SetvarKind::Fraction => i32::from(current == 0),
            SetvarKind::Choice if !self.values.is_empty() => {
                let at = self.values.iter().position(|v| *v == current).unwrap_or(0);
                let n = self.values.len() as i32;
                let to = (at as i32 + delta).rem_euclid(n) as usize;
                self.values[to]
            }
            SetvarKind::Choice | SetvarKind::Number => {
                (current + delta).clamp(self.min_val, self.max_val)
            }
        }
    }
}

/// Message key of the label of a known option.
fn known_label_key(name: &str) -> Option<&'static str> {
    use opts_gui::l10n::wizard_keys::wiz;
    Some(match name {
        "vis_wimpel_invisible" => wiz::WIZ_OPTION_VIS_WIMPEL_INVISIBLE,
        "wimpel_visibility" => wiz::WIZ_OPTION_WIMPEL_VISIBILITY,
        "vis_grill_invisible" => wiz::WIZ_OPTION_VIS_GRILL_INVISIBLE,
        "ident_visible" => wiz::WIZ_OPTION_IDENT_VISIBLE,
        _ => return None,
    })
}

/// The option's own name made readable (`vis_front_light` becomes `Vis front light`).
pub fn humanise(name: &str) -> String {
    opts_core::setvars::humanise(name)
}

/// The catalogue of a vehicle's setvars: its repaints' `[setvar]` lines, the variables it declares
/// and the vendor's documentation of its family (M-0078).
pub fn catalogue_of(reps: &VehicleRepaints) -> SetvarCatalogue {
    let observed: Vec<CtiSetvar> = reps
        .setvars
        .iter()
        .flat_map(|(name, lines)| {
            lines.iter().map(move |(variable, value)| CtiSetvar {
                repaint: name.clone(),
                variable: variable.clone(),
                value: value.clone(),
            })
        })
        .collect();
    SetvarCatalogue::build(&observed, &reps.declared, family_of_folder(&reps.folder))
}

/// The options of a vehicle with their current values: `custom` holds the user's changes,
/// `repaint` is the chosen repaint (empty: the vehicle's default) whose own values are the
/// defaults. Decimal options (door dynamics) are not offered; the repaints keep setting them.
pub fn setvars_from(
    reps: &VehicleRepaints,
    custom: &HashMap<String, i32>,
    repaint: &str,
) -> Vec<SetvarItem> {
    let loc = Localization::detect();
    let repaint = if repaint.is_empty() {
        reps.default_name.as_str()
    } else {
        repaint
    };
    let catalogue = catalogue_of(reps);
    catalogue
        .entries
        .iter()
        .filter(|e| e.kind != SetvarKind::Fraction)
        .map(|e| {
            let default_value = e.default_for(repaint).round() as i32;
            let value = custom.get(&e.name).copied().unwrap_or(default_value);
            let mut values: Vec<i32> = e.values.iter().map(|v| *v as i32).collect();
            if e.kind == SetvarKind::Choice && !values.contains(&value) {
                values.push(value);
                values.sort_unstable();
            }
            SetvarItem {
                label: known_label_key(&e.name).map_or_else(|| e.label.clone(), |k| loc.t(k)),
                description: e.description.clone(),
                value,
                default_value,
                min_val: e.min.round() as i32,
                max_val: e.max.round() as i32,
                kind: e.kind,
                values,
                group: e.group.clone(),
                unverified: e.range_unverified,
                name: e.name.clone(),
            }
        })
        .collect()
}

/// The options of a menu vehicle (reads its repaints; used by the in-game vehicle menu).
pub fn get_vehicle_setvars(
    vehicle: &VehicleItem,
    custom_setvars: &HashMap<String, i32>,
    repaint: &str,
) -> Vec<SetvarItem> {
    setvars_from(&vehicle_repaints(vehicle), custom_setvars, repaint)
}

#[cfg(test)]
mod tests;
