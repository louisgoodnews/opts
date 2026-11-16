//! Column 3 of the Select Vehicle modal: the free entry points and the bus stops.

use super::*;

/// Column 3: the free entry points and the bus stops.
pub(super) fn entry_column(
    row: &mut ChildSpawnerCommands,
    menu: &VehicleMenu,
    entries_resolved: (usize, bool),
) {
    let c3 = column(row, 30.0);
    row.commands().entity(c3).with_children(|c| {
        label(c, "Entry points", 15.0, ColorRole::Primary);
        if !entries_resolved.1 {
            label(
                c,
                format!(
                    "Locating entry points… {} of {} found; the others cannot be checked for blocking yet",
                    entries_resolved.0, menu.entry_total
                ),
                12.0,
                ColorRole::OnSurfaceVariant,
            );
        }
        scroll_list(c, 2, menu.scroll[2], Val::Auto, |list| {
            if menu.shown_entries.is_empty() {
                label(
                    list,
                    "No free entry point (all are blocked by vehicles).",
                    12.0,
                    ColorRole::OnSurfaceVariant,
                );
            }
            for (id, name) in &menu.shown_entries {
                let selected = menu.entry == Some(*id);
                button(list, name, selected, Action::PickEntry(*id));
            }
            // Bus stops: the vehicle stands with its front at the stop, if it is free.
            if !menu.shown_stops.is_empty() {
                label(list, "Bus stops", 15.0, ColorRole::Primary);
            }
            for (id, name) in &menu.shown_stops {
                let selected = menu.stop == Some(*id);
                button(list, name, selected, Action::PickStop(*id));
            }
        });
    });
}
