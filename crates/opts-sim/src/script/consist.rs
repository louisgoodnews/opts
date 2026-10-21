//! Control cables between the units of a coupled vehicle (`[control_cable_front]` and
//! `[control_cable_back]`): the electrical lines that carry the tram's battery switch, indicators,
//! door commands and traction demand from one car's scripts to the others'.
//!
//! A cable is declared by every unit it passes through, under the same kind and number, with two
//! variables: the unit's scripts write one to put a value on the line (`send`) and read the other
//! for the value the line carries (`receive`). The line is live between two neighbouring units
//! while the coupling variable of the declaring sides is above 0.5. Every unit of a connected run
//! of units sees what the others put on it: the value of largest magnitude wins, a unit's own
//! value reaches itself too (the scripts never write the variable they read, and a car at the end
//! of a train, with nothing behind it, still gets its own battery switch back).

use std::collections::BTreeSet;

use super::ScriptEngine;

/// One declared cable of a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cable {
    /// Declared at the unit's front coupling (else at the rear one).
    pub front: bool,
    /// Plug kind: `C`, `L` or `R`.
    pub kind: char,
    /// Number within the kind.
    pub index: u32,
    /// Variable that receives the carried value.
    pub receive: String,
    /// Variable the scripts write to put a value on the line.
    pub send: String,
    /// The link works while this variable of the unit is above 0.5.
    pub coupling: String,
}

/// A unit's side of the bridge: its script engine (a unit without scripts passes the line through
/// without taking part) and its declared cables.
pub struct Unit<'a> {
    /// The unit's engine, `None` when its scripts run in another unit's.
    pub engine: Option<&'a mut ScriptEngine>,
    /// The cables the unit's vehicle file declares.
    pub cables: &'a [Cable],
}

fn find(cables: &[Cable], front: bool, kind: char, index: u32) -> Option<&Cable> {
    cables
        .iter()
        .find(|c| c.front == front && c.kind == kind && c.index == index)
}

impl Unit<'_> {
    /// Whether the unit's `side` (front or rear) plug of this cable is connected.
    fn plugged(&self, front: bool, key: (char, u32)) -> bool {
        find(self.cables, front, key.0, key.1).is_some_and(|c| {
            self.engine
                .as_deref()
                .is_none_or(|e| e.get(&c.coupling) > 0.5)
        })
    }

    fn declares(&self, key: (char, u32)) -> Option<&Cable> {
        find_any(self.cables, key)
    }
}

/// Copies every cable's value across the units, front to back in `units`; returns how many
/// receive variables were written.
pub fn bridge(units: &mut [Unit<'_>]) -> usize {
    let keys: BTreeSet<(char, u32)> = units
        .iter()
        .flat_map(|u| u.cables.iter().map(|c| (c.kind, c.index)))
        .collect();
    let mut written = 0;
    for key in keys {
        let mut start = 0;
        while start < units.len() {
            if units[start].declares(key).is_none() {
                start += 1;
                continue;
            }
            // The run of units the line connects: each declares it and the plugs between touch.
            let mut end = start + 1;
            while end < units.len()
                && units[end].declares(key).is_some()
                && units[end - 1].plugged(false, key)
                && units[end].plugged(true, key)
            {
                end += 1;
            }
            let mut value = 0.0f32;
            for u in &units[start..end] {
                if let (Some(c), Some(e)) = (u.declares(key), u.engine.as_deref()) {
                    let v = e.get(&c.send);
                    if v.abs() > value.abs() {
                        value = v;
                    }
                }
            }
            for u in &mut units[start..end] {
                if let (Some(c), Some(e)) = (find_any(u.cables, key), u.engine.as_deref_mut()) {
                    e.set(&c.receive, value);
                    written += 1;
                }
            }
            start = end;
        }
    }
    written
}

fn find_any(cables: &[Cable], key: (char, u32)) -> Option<&Cable> {
    find(cables, true, key.0, key.1).or_else(|| find(cables, false, key.0, key.1))
}

#[cfg(test)]
mod tests;
