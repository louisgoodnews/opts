//! Equipment options of a vehicle: the `[setvar]` lines of its repaints, with plain-language
//! labels.

use std::collections::{BTreeMap, HashMap};

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
    /// The value a repaint gives it.
    pub default_value: i32,
    /// Smallest value.
    pub min_val: i32,
    /// Largest value.
    pub max_val: i32,
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
    let spaced = name.replace(['_', '-'], " ");
    let mut chars = spaced.trim().chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// The options of discovered repaints with their current values.
pub fn setvars_from(reps: &VehicleRepaints, custom: &HashMap<String, i32>) -> Vec<SetvarItem> {
    let loc = Localization::detect();
    let mut found = BTreeMap::new();
    for (name, value) in reps.setvars.values().flatten() {
        found
            .entry(name.clone())
            .or_insert_with(|| value.parse::<i32>().unwrap_or(1));
    }
    let mut items: Vec<SetvarItem> = found
        .into_iter()
        .map(|(name, default_value)| SetvarItem {
            label: known_label_key(&name).map_or_else(|| humanise(&name), |k| loc.t(k)),
            description: String::new(),
            value: custom.get(&name).copied().unwrap_or(default_value),
            default_value,
            min_val: 0,
            max_val: 1,
            name,
        })
        .collect();
    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}

/// The options of a menu vehicle (reads its repaints; used by the in-game vehicle menu).
pub fn get_vehicle_setvars(
    vehicle: &VehicleItem,
    custom_setvars: &HashMap<String, i32>,
) -> Vec<SetvarItem> {
    setvars_from(&vehicle_repaints(vehicle), custom_setvars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_options_are_named_by_their_own_name_not_by_a_generic_phrase() {
        assert_eq!(humanise("vis_front_light"), "Vis front light");
        let mut reps = VehicleRepaints::default();
        reps.setvars.insert(
            "Beige".into(),
            vec![
                ("vis_front_light".into(), "1".into()),
                ("ident_visible".into(), "0".into()),
            ],
        );
        let items = setvars_from(&reps, &HashMap::from([("vis_front_light".to_string(), 0)]));
        assert_eq!(items.len(), 2);
        let front = items
            .iter()
            .find(|i| i.name == "vis_front_light")
            .expect("item");
        assert_eq!(
            (front.label.as_str(), front.value, front.default_value),
            ("Vis front light", 0, 1)
        );
        let ident = items
            .iter()
            .find(|i| i.name == "ident_visible")
            .expect("item");
        assert_ne!(ident.label, "Vehicle Equipment Option");
        assert!(!ident.label.contains('⟦'), "known options have a message");
    }
}
