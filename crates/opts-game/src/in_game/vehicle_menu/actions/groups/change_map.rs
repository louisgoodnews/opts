//! The change-of-map actions of the menu: picking a map and its start point, and confirming.

use super::super::*;

/// The change-of-map pages. Returns the action when this group does not handle it.
pub(in super::super) fn change_map_action(action: Action, c: &mut MenuCtx) -> Option<Action> {
    match action {
        Action::PickMap(id) => {
            if c.menu.change_map.map.as_deref() != Some(id.as_str()) {
                c.menu.change_map.entry = c
                    .menu
                    .change_map
                    .entry
                    .take()
                    .filter(|_| c.menu.page == Page::ChangeMap);
            }
            c.menu.change_map.map = Some(id);
            c.menu.page = Page::ChangeMapConfirm;
            c.menu.status.clear();
            None
        }
        Action::PickMapEntry(name) => {
            c.menu.change_map.entry = Some(name);
            None
        }
        Action::DriveWithoutVehicle => {
            c.menu.change_map.without_vehicle = true;
            None
        }
        Action::ConfirmChangeMap => {
            c.commands.queue(|w: &mut World| confirm_change_map(w));
            None
        }
        other => Some(other),
    }
}
