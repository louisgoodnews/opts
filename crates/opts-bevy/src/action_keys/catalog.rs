//! The built-in action catalogue: each action with its group and default chord.

use super::*;

/// One built-in action.
#[derive(Debug, Clone, Copy)]
pub struct ActionDef {
    /// Short name (`help`), the argument of [`ActionKeys`] and the last part of the setting.
    pub id: &'static str,
    /// Group (`ui`, `map`, `drive`), the middle part of the setting `controls.<group>.<id>`.
    pub group: &'static str,
    /// The default chord as setting text.
    pub default: &'static str,
    /// Which vehicles the action is for.
    pub scope: ActionScope,
}

/// The vehicles an action is for: a key of the rail controls may share its default with a road
/// vehicle's key (Q is the bus microphone and the tram's lever).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionScope {
    /// Every vehicle.
    All,
    /// Road vehicles (buses, trolleybuses).
    Road,
    /// Rail vehicles (trams, trains).
    Rail,
}

pub(super) const fn def(group: &'static str, id: &'static str, default: &'static str) -> ActionDef {
    ActionDef {
        id,
        group,
        default,
        scope: ActionScope::All,
    }
}

pub(super) const fn road(
    group: &'static str,
    id: &'static str,
    default: &'static str,
) -> ActionDef {
    ActionDef {
        id,
        group,
        default,
        scope: ActionScope::Road,
    }
}

pub(super) const fn rail(
    group: &'static str,
    id: &'static str,
    default: &'static str,
) -> ActionDef {
    ActionDef {
        id,
        group,
        default,
        scope: ActionScope::Rail,
    }
}

/// The actions, in the order of the settings page.
pub const ACTIONS: &[ActionDef] = &[
    road("drive", "parking_brake", "Period"),
    // Fills the fuel or battery of the vehicle (M-0123); F12 is not used by any other action.
    def("drive", "refuel", "F12"),
    // The controls of rail vehicles: no pedals and no parking brake. W and S choose the
    // direction of travel, Q and Y (the key at the Z position of a QWERTY keyboard) move the
    // power lever up (towards traction) and down (towards braking), the arrows choose the way at
    // the next point.
    rail("drive", "rail_forward", "W"),
    rail("drive", "rail_reverse", "S"),
    rail("drive", "rail_lever_up", "Q"),
    rail("drive", "rail_lever_down", "Z"),
    rail("drive", "rail_lever_zero", "X"),
    rail("drive", "rail_switch_toggle", "G"),
    // The other cab of a tram (A and B) or of a traction of trains.
    rail("drive", "rail_switch_cab", "F3"),
    rail("drive", "rail_branch_left", "ArrowLeft"),
    rail("drive", "rail_branch_straight", "ArrowUp"),
    rail("drive", "rail_branch_right", "ArrowRight"),
    def("map", "minimap_toggle", "F7"),
    def("map", "minimap_rotate", "Shift+F7"),
    def("map", "minimap_speed_zoom", "Ctrl+F7"),
    def("map", "minimap_zoom_in", "Equal"),
    def("map", "minimap_zoom_out", "Minus"),
    def("map", "full_map", "F6"),
    def("camera", "door_camera", "F4"),
    def("ui", "help", "F1"),
    def("ui", "theme", "F2"),
    def("ui", "quick_save", "F5"),
    def("ui", "quick_load", "F9"),
    def("ui", "switch_vehicle", "F8"),
    def("ui", "switch_vehicle_prev", "Shift+F8"),
    def("ui", "developer_panel", "F10"),
    def("ui", "developer_panel_detail", "Shift+F10"),
];

/// The setting that holds the chord of an action.
#[must_use]
pub fn setting_of(def: &ActionDef) -> String {
    format!("controls.{}.{}", def.group, def.id)
}

/// The definition of `id`.
#[must_use]
pub fn action(id: &str) -> Option<&'static ActionDef> {
    ACTIONS.iter().find(|a| a.id == id)
}
