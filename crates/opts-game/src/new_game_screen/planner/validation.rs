//! Whether the plan's choices make a valid drive, and how to make them valid.

use super::super::types::{GameCatalogue, NewGameSetup};

/// Why a drive cannot start (the reason shown on the disabled control).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// No map is chosen or it is not in the catalogue.
    NoMap,
    /// The chosen map has start points but none is chosen.
    NoStartPoint,
    /// No vehicle is chosen or it is not in the catalogue.
    NoVehicle,
}

/// Whether the choices make a valid drive.
///
/// # Errors
/// The first problem found, in the order of the steps.
pub fn validate(setup: &NewGameSetup, catalogue: &GameCatalogue) -> Result<(), Invalid> {
    let Some(map) = catalogue
        .maps
        .iter()
        .find(|m| m.friendly_name == setup.selected_map)
    else {
        return Err(Invalid::NoMap);
    };
    if !map.entry_points.is_empty() && !map.entry_points.contains(&setup.selected_entry_point) {
        return Err(Invalid::NoStartPoint);
    }
    // The key is the display name, or the id when two vehicles share that name.
    if !setup.no_vehicle && catalogue.find_vehicle(&setup.selected_vehicle).is_none() {
        return Err(Invalid::NoVehicle);
    }
    Ok(())
}

/// Makes the selection valid when the catalogue allows it: an unknown map becomes the first
/// map (with its first start point), an unknown start point the map's first, an unknown vehicle
/// the preferred default (`MAN_SD77`) or the first. Nothing happens while the lists are empty.
pub fn ensure_valid(setup: &mut NewGameSetup, catalogue: &GameCatalogue) {
    if let Some(map) = catalogue.map_by_name(&setup.selected_map) {
        if !map.entry_points.is_empty() && !map.entry_points.contains(&setup.selected_entry_point) {
            setup.selected_entry_point.clone_from(&map.entry_points[0]);
        }
    } else if let Some(map) = catalogue.maps.first() {
        setup.selected_map.clone_from(&map.friendly_name);
        setup.selected_entry_point = map.entry_points.first().cloned().unwrap_or_default();
    }
    if !setup.no_vehicle && catalogue.find_vehicle(&setup.selected_vehicle).is_none() {
        if let Some(v) = catalogue
            .vehicles
            .iter()
            .find(|v| v.id.eq_ignore_ascii_case("MAN_SD77"))
            .or_else(|| catalogue.vehicles.first())
        {
            setup.selected_vehicle = catalogue.vehicle_key(v);
            setup.selected_livery.clear();
        }
    }
}
