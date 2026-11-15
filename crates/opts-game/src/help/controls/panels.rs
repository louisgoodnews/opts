//! The camera, map and menu groups of the controls table.

use bevy::prelude::KeyCode;

use super::Group;
use super::table::{
    CAMERA, DEV, DOOR_CAMERA, EGO, FULLMAP, HELP, MINIMAP, PAUSE, QUICK_SAVE, SESSION, SIM_PAUSE,
    THEME, VIRTUAL_MIRRORS, bind, rebindable,
};
use crate::shell::keys;

pub(super) const CAMERA_GROUP: Group = (
    keys::HELP_GROUP_CAMERA,
    &[
        bind(
            "1 – 5",
            &[
                KeyCode::Digit1,
                KeyCode::Digit2,
                KeyCode::Digit3,
                KeyCode::Digit4,
                KeyCode::Digit5,
            ],
            keys::HELP_KEY_CAMERA,
            CAMERA,
        ),
        bind("0", &[KeyCode::Digit0], keys::HELP_KEY_CAMERA_PREV, CAMERA),
        bind(
            "Shift + Space",
            &[KeyCode::ShiftLeft, KeyCode::Space],
            keys::HELP_KEY_CAMERA_RESET,
            CAMERA,
        ),
        bind(
            "Tab",
            &[KeyCode::Tab],
            keys::HELP_KEY_VIRTUAL_MIRRORS,
            VIRTUAL_MIRRORS,
        ),
    ],
);

pub(super) const MAP_GROUP: Group = (
    keys::HELP_GROUP_MAP,
    &[
        rebindable(
            &["minimap_toggle"],
            "F7",
            &[KeyCode::F7],
            keys::HELP_KEY_MINIMAP,
            MINIMAP,
        ),
        rebindable(
            &["minimap_zoom_in", "minimap_zoom_out"],
            "+ / -",
            &[KeyCode::Equal, KeyCode::Minus],
            keys::HELP_KEY_MINIMAP_ZOOM,
            MINIMAP,
        ),
        rebindable(
            &["full_map"],
            "F6",
            &[KeyCode::F6],
            keys::HELP_KEY_FULLMAP,
            FULLMAP,
        ),
        rebindable(
            &["door_camera"],
            "F4",
            &[KeyCode::F4],
            keys::HELP_KEY_DOOR_CAMERA,
            DOOR_CAMERA,
        ),
    ],
);

pub(super) const MENUS_GROUP: Group = (
    keys::HELP_GROUP_MENUS,
    &[
        bind("Esc", &[KeyCode::Escape], keys::HELP_KEY_PAUSE_MENU, PAUSE),
        bind("P", &[KeyCode::KeyP], keys::HELP_KEY_SIM_PAUSE, SIM_PAUSE),
        bind("Alt", &[KeyCode::AltLeft], keys::HELP_KEY_VEHICLE_MENU, EGO),
        rebindable(
            &["switch_vehicle"],
            "F8",
            &[KeyCode::F8],
            keys::HELP_KEY_SWITCH_VEHICLE,
            SESSION,
        ),
        rebindable(
            &["quick_save"],
            "F5",
            &[KeyCode::F5],
            keys::HELP_KEY_QUICK_SAVE,
            QUICK_SAVE,
        ),
        rebindable(
            &["quick_load"],
            "F9",
            &[KeyCode::F9],
            keys::HELP_KEY_QUICK_LOAD,
            QUICK_SAVE,
        ),
        rebindable(&["help"], "F1", &[KeyCode::F1], keys::HELP_KEY_HELP, HELP),
        rebindable(
            &["theme"],
            "F2",
            &[KeyCode::F2],
            keys::HELP_KEY_THEME,
            THEME,
        ),
        rebindable(
            &["developer_panel", "developer_panel_detail"],
            "F10",
            &[KeyCode::F10],
            keys::HELP_KEY_DEV,
            DEV,
        ),
    ],
);
