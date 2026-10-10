//! Vehicle setvar discovery routines: inspecting .cti files and known model definitions.

use std::fs;
use std::path::Path;

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
    let mut scanned_vars = std::collections::HashSet::new();
    for base in [
        "/home/louisgoodnews/nvme2n1/steamapps/common/OMSI 2/Vehicles",
        "/home/louisgoodnews/.steam/steam/steamapps/common/OMSI 2/Vehicles",
    ] {
        let base_path = Path::new(base);
        if !base_path.is_dir() {
            continue;
        }

        let rel_folder = if vehicle.id.contains("EN92")
            || vehicle.id.contains("GN92")
            || vehicle.model.contains("NL202")
        {
            Some("MAN_NL_NG")
        } else if vehicle.id.contains("SD") || vehicle.model.contains("SD200") {
            Some("MAN_SD200")
        } else if vehicle.model.contains("SD202") || vehicle.id.contains("D8") {
            Some("MAN_SD202")
        } else {
            None
        };

        if let Some(folder) = rel_folder {
            let tex_dir = base_path.join(folder).join("Texture");
            if let Ok(entries) = fs::read_dir(&tex_dir) {
                for entry in entries.flatten() {
                    if let Ok(ctis) = fs::read_dir(entry.path()) {
                        for cti in ctis.flatten() {
                            if cti.path().extension().and_then(|e| e.to_str()) == Some("cti") {
                                if let Ok(txt) = fs::read_to_string(cti.path()) {
                                    let lines: Vec<&str> = txt.lines().map(|l| l.trim()).collect();
                                    for i in 0..lines.len() {
                                        if lines[i] == "[setvar]" && i + 2 < lines.len() {
                                            let v_name = lines[i + 1].to_string();
                                            let def: i32 = lines[i + 2].parse().unwrap_or(1);
                                            scanned_vars.insert((v_name, def));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
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

    if items.is_empty()
        && (vehicle.manufacturer == "MAN"
            || vehicle.model.contains("NL")
            || vehicle.model.contains("SD"))
    {
        for (name, label, desc, def) in [
            (
                "vis_wimpel_invisible",
                "Festive Bunting Flag",
                "Toggles front bunting pennant on the vehicle roof",
                0,
            ),
            (
                "vis_grill_invisible",
                "Front Grille Cover",
                "Toggles classic front radiator protective grille trim",
                0,
            ),
            (
                "display_ext",
                "Exterior Destination Matrix",
                "0 = High-res Flipdot Matrix, 1 = Classic Rollband",
                0,
            ),
            (
                "air_conditioning",
                "Rooftop Air Conditioning",
                "0 = Roof Ventilation Hatches, 1 = ThermoKing Rooftop A/C",
                1,
            ),
        ] {
            let current_val = *custom_setvars.get(name).unwrap_or(&def);
            items.push(SetvarItem {
                name: name.to_string(),
                label: label.to_string(),
                description: desc.to_string(),
                value: current_val,
                min_val: 0,
                max_val: 1,
            });
        }
    }

    items.sort_by(|a, b| a.label.cmp(&b.label));
    items
}
