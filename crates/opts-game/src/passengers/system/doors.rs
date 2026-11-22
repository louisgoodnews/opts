//! Door state of the bus: whether a door is open, and the door readings of the vehicle scripts.

use super::*;

/// Whether door `n` of kind `kind` (`entry`/`exit`) is open in the engine of `store`.
pub(super) fn door_open(
    engines: &Query<&VehicleScripts>,
    (store, n): (Entity, usize),
    kind: &str,
) -> bool {
    engines
        .get(store)
        .ok()
        .and_then(|s| s.engine.lock().ok())
        .is_some_and(|e| e.get(&format!("pax_{kind}{n}_open")) > 0.5)
}

/// The doors the vehicle's scripts report: each unit's in the engine of its own scripts.
pub(super) fn read_doors(
    engines: &Query<&VehicleScripts>,
    entries: &[(Entity, usize)],
    exits: &[(Entity, usize)],
) -> Doors {
    Doors {
        entries: entries
            .iter()
            .map(|&d| door_open(engines, d, "entry"))
            .collect(),
        exits: exits
            .iter()
            .map(|&d| door_open(engines, d, "exit"))
            .collect(),
    }
}

#[cfg(test)]
mod tests;
