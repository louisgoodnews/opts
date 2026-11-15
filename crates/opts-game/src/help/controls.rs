//! The controls table, generated from [`GROUPS`], the one registry of the keyboard bindings that
//! Help prints. A test checks every listed key against the input systems' source files.

mod driving;
mod panels;
mod spawn;
mod table;

use bevy::prelude::*;

pub use spawn::spawn_controls;
pub use table::GROUPS;

/// One binding: what is shown, the keys behind it, what it does and where it is handled.
// `keys` and `source` document the binding and are checked by the tests below.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    /// The keys as printed (key names are not translated).
    pub shown: &'static str,
    /// Every key the binding uses; [`tests`] check each is read by `source`.
    pub keys: &'static [KeyCode],
    /// Message key of what the keys do.
    pub action: &'static str,
    /// The source file (repository path) whose input system reads the keys.
    pub source: &'static str,
    /// The rebindable actions behind the keys (`opts_bevy::action_keys`): the chip shows their
    /// current chords, `shown` is the default; empty for keys that cannot move.
    pub ids: &'static [&'static str],
}

/// The chip of a rebindable row: its text follows the player's bindings.
#[derive(Component, Debug, Clone, Copy)]
pub struct ActionChip(pub &'static [&'static str]);

/// Writes the current chords of the rebindable rows into their chips.
pub fn sync_action_chips(
    keys: Option<Res<opts_bevy::ActionKeys>>,
    mut chips: Query<(&ActionChip, &mut Text)>,
    added: Query<(), Added<ActionChip>>,
) {
    let changed = keys.as_ref().is_some_and(|k| k.is_changed()) || !added.is_empty();
    if !changed {
        return;
    }
    let keys = opts_bevy::action_keys_of(&keys);
    for (chip, mut text) in &mut chips {
        let shown: Vec<String> = chip.0.iter().map(|id| keys.label(id)).collect();
        let shown = shown.join(" / ");
        if text.0 != shown {
            text.0 = shown;
        }
    }
}

/// A group: its message key and bindings.
pub type Group = (&'static str, &'static [Binding]);

#[cfg(test)]
mod tests;
