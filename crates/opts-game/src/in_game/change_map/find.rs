//! Finding the target map, its entry point and the vehicle to carry over.

use super::*;

/// The maps whose id, folder or friendly name equals `text` (case-insensitively), else those that
/// contain it.
#[must_use]
pub fn find_maps<'c>(catalogue: &'c GameCatalogue, text: &str) -> Vec<&'c MapItem> {
    let t = text.trim().to_lowercase();
    let exact: Vec<&MapItem> = catalogue
        .maps
        .iter()
        .filter(|m| {
            [&m.id, &m.name, &m.friendly_name]
                .iter()
                .any(|n| n.to_lowercase() == t)
        })
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    catalogue
        .maps
        .iter()
        .filter(|m| {
            [&m.id, &m.name, &m.friendly_name]
                .iter()
                .any(|n| n.to_lowercase().contains(&t))
        })
        .collect()
}

/// The entry point of `map` that `text` names (exact, else the only one that contains it).
pub(super) fn find_entry<'m>(map: &'m MapItem, text: &str) -> Option<&'m String> {
    let t = text.trim().to_lowercase();
    map.entry_points
        .iter()
        .find(|e| e.to_lowercase() == t)
        .or_else(|| {
            let mut hits = map
                .entry_points
                .iter()
                .filter(|e| e.to_lowercase().contains(&t));
            let first = hits.next()?;
            hits.next().is_none().then_some(first)
        })
}

pub(super) fn find_vehicle<'c>(
    catalogue: &'c GameCatalogue,
    record: &SessionVehicleRecord,
) -> Option<&'c VehicleItem> {
    let by_id = |id: &str| {
        catalogue
            .vehicles
            .iter()
            .find(|v| !id.is_empty() && v.id.eq_ignore_ascii_case(id))
    };
    by_id(&record.catalogue_id).or_else(|| {
        catalogue
            .vehicles
            .iter()
            .find(|v| v.full_name == record.name)
    })
}
