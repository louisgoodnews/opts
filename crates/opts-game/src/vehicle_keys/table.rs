//! The table of the rebindable vehicle key actions, in the order of the settings page.

use super::{Scope, VehicleKeyAction};

macro_rules! action {
    ($id:literal, $trigger:expr, $default:literal, $held:literal) => {
        action!($id, $trigger, $default, $held, Scope::Both, None)
    };
    ($id:literal, $trigger:expr, $default:literal, $held:literal, $scope:expr, $also:expr) => {
        VehicleKeyAction {
            id: $id,
            setting: concat!("controls.vehicle.", $id),
            rail_setting: match $scope {
                Scope::Both => Some(concat!("controls.rail.", $id)),
                _ => None,
            },
            label_key: concat!("set-vk-", $id),
            desc_key: concat!("set-vk-", $id, "-desc"),
            trigger: $trigger,
            default: $default,
            held: $held,
            scope: $scope,
            also: $also,
        }
    };
}

/// The actions, in the order of the settings page.
pub const ACTIONS: &[VehicleKeyAction] = &[
    action!("battery", "cp_batterietrennschalter_toggle", "E", true),
    action!("engine_start", "kw_m_enginestart", "M", true),
    action!("lights", "kw_scheinwerfer_toggle", "L", false),
    action!("parking_light", "kw_standlicht_toggle", "Shift+L", false),
    action!("high_beam", "kw_fernlicht_toggle", "F", false),
    action!("fog_rear", "taster_nebelschluss", "Ctrl+F", false),
    action!("indicator_left", "blinker_left_set", "Numpad7", true),
    action!("indicator_right", "blinker_right_set", "Numpad9", true),
    action!("indicator_off", "blinker_off", "NumpadDecimal", false),
    action!("hazard", "blinker_warn_toggle", "B", false),
    action!("horn", "horn", "H", true),
    action!("wiper", "kw_wipermode_up", "none", false),
    // Buses: the doors one by one and all at once; the trams only clear the doors and choose the
    // side (`bus_doorfront0/1` of a tram are its open and close buttons, which stay on the mouse).
    action!(
        "door_1",
        "bus_doorfront0",
        "NumpadDivide",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_2",
        "bus_doorfront1",
        "NumpadMultiply",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_3",
        "bus_doorfront2",
        "Ctrl+NumpadDivide",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_4",
        "bus_doorfront3",
        "Ctrl+NumpadMultiply",
        false,
        Scope::Bus,
        None
    ),
    action!(
        "door_clearance",
        "bus_dooraft",
        "NumpadSubtract",
        false,
        Scope::Both,
        None
    ),
    action!(
        "door_central",
        "bus_doorfront4",
        "NumpadAdd",
        false,
        Scope::Bus,
        Some("bus_doorfront5")
    ),
    action!(
        "door_side",
        opts_bevy::DOOR_SIDE_TOGGLE,
        "NumpadMultiply",
        false,
        Scope::Tram,
        None
    ),
    action!(
        "cabin_light",
        opts_bevy::CABIN_LIGHT_TOGGLE,
        "Ctrl+L",
        false,
        Scope::Both,
        None
    ),
    action!("interior_driver", "cp_fahrerlicht_toggle", "6", false),
    action!(
        "interior_lower_right",
        "cp_licht_untenrechts_toggle",
        "7",
        false
    ),
    action!("interior_upper", "cp_licht_oberdeck_toggle", "8", false),
    action!("interior_lower", "cp_licht_unterdeck_toggle", "9", false),
    action!("microphone", "cp_microphone", "Q", true, Scope::Bus, None),
    action!("ticket", "ticket_give", "T", false),
    action!("ticket_change", "change_give", "Ctrl+T", false),
    action!("ticket_take", "change_take", "Shift+T", false),
];
