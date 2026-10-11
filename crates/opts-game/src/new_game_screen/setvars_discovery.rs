//! Vehicle setvar discovery routines: inspecting .cti files and known model definitions.

use crate::new_game_screen::types::VehicleItem;

/// A configurable setvar item for an OMSI vehicle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetvarItem {
    pub name: String,
    pub label: String,
    pub description: String,
    pub value: i32,
    pub min_val: i32,
    pub max_val: i32,
}

/// Discovers setvars for a vehicle by inspecting .cti files and known model definitions.
pub fn get_vehicle_setvars(
    vehicle: &VehicleItem,
    custom_setvars: &std::collections::HashMap<String, i32>,
) -> Vec<SetvarItem> {
    // Every `[setvar]` of the vehicle's repaint presets: the equipment options a repaint switches.
    let mut scanned_vars = std::collections::BTreeMap::new();
    for (name, value) in crate::new_game_screen::repaints::vehicle_repaints(vehicle)
        .setvars
        .into_values()
        .flatten()
    {
        scanned_vars
            .entry(name)
            .or_insert_with(|| value.parse::<i32>().unwrap_or(1));
    }

    let mut items = Vec::new();
    for (name, def_val) in scanned_vars {
        let current_val = *custom_setvars.get(&name).unwrap_or(&def_val);
        let (label, desc) = match name.as_str() {
            "vis_wimpel_invisible" => (
                "Bunting / Pennant Visibility",
                "Toggles decorative front bunting flag on roof",
            ),
            "wimpel_visibility" => (
                "Bunting Flag Mount",
                "Toggles festive flag mount accessories",
            ),
            "vis_grill_invisible" => (
                "Front Radiator Grille Cover",
                "Toggles front grille shield and badge protector",
            ),
            "ident_visible" => (
                "Fleet Identification Decal",
                "Toggles external operator identification labels",
            ),
            _ => (
                "Vehicle Equipment Option",
                "Dynamic vehicle equipment toggle defined in repaint",
            ),
        };
        items.push(SetvarItem {
            name,
            label: label.to_string(),
            description: desc.to_string(),
            value: current_val,
            min_val: 0,
            max_val: 1,
        });
    }

    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}
