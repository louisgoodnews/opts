//! Cab queries: which scripts a cab runs, whether it is turned, and the next cab.

use super::*;


/// The scripts of the active cab: the unit's own engine when a unit is active and has one, else
/// the head's.
#[must_use]
pub fn cab_scripts<'a>(
    head: &'a VehicleScripts,
    active: Option<ActiveCab>,
    parts: &'a Query<&VehicleScripts, With<ScriptPart>>,
) -> &'a VehicleScripts {
    active
        .and_then(|a| a.0)
        .and_then(|e| parts.get(e).ok())
        .unwrap_or(head)
}

/// Whether the active cab is a unit that hangs backwards in the train (a tram's head B): driven
/// from there, the vehicle's forward is the head's backward.
#[must_use]
pub fn cab_is_turned(
    active: Option<&ActiveCab>,
    consist: Option<&crate::consist_scripts::ScriptConsist>,
) -> bool {
    active
        .and_then(|a| a.0)
        .and_then(|e| consist?.units.iter().find(|u| u.part == Some(e)))
        .is_some_and(|u| u.turned)
}

/// The next cab after `current` among `cabs` (wrapping); the first when `current` is not one.
#[must_use]
pub fn next_cab(cabs: &[Option<Entity>], current: Option<Entity>) -> Option<Option<Entity>> {
    if cabs.len() < 2 {
        return None;
    }
    let at = cabs.iter().position(|c| *c == current);
    Some(cabs[at.map_or(0, |i| (i + 1) % cabs.len())])
}

/// Whether a unit's engine is a cab's: it answers the keys of the driver's seat. (Every unit of
/// an NF6D file lists the driver cameras, so the cameras do not tell the cabs from the joints.)
pub(super) fn is_cab_engine(scripts: &VehicleScripts) -> bool {
    scripts
        .engine
        .lock()
        .is_ok_and(|e| e.has_trigger("automatic_d") || e.has_trigger("kw_s_n"))
}
