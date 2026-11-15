//! The driving and rail groups of the controls table.

use bevy::prelude::KeyCode;

use super::Group;
use super::table::{RAIL, RAIL_CONTROLS, SIM, bind, rebindable};
use crate::shell::keys;

pub(super) const DRIVING_GROUP: Group = (
    keys::HELP_GROUP_DRIVING,
    &[
        bind(
            "W / S",
            &[KeyCode::KeyW, KeyCode::KeyS],
            keys::HELP_KEY_THROTTLE,
            SIM,
        ),
        bind(
            "← / →",
            &[KeyCode::ArrowLeft, KeyCode::ArrowRight],
            keys::HELP_KEY_STEER,
            SIM,
        ),
        rebindable(
            &["parking_brake"],
            ".",
            &[KeyCode::Period],
            keys::HELP_KEY_PARKING,
            SIM,
        ),
        bind(
            "D / N / R",
            &[KeyCode::KeyD, KeyCode::KeyN, KeyCode::KeyR],
            keys::HELP_KEY_GEARS,
            SIM,
        ),
    ],
);

pub(super) const RAIL_GROUP: Group = (
    keys::HELP_GROUP_RAIL,
    &[
        rebindable(
            &["rail_forward", "rail_reverse"],
            "W / S",
            &[KeyCode::KeyW, KeyCode::KeyS],
            keys::HELP_KEY_RAIL_DIRECTION,
            RAIL_CONTROLS,
        ),
        rebindable(
            &["rail_lever_up", "rail_lever_down"],
            "Q / Y",
            &[KeyCode::KeyQ, KeyCode::KeyZ],
            keys::HELP_KEY_RAIL_LEVER,
            RAIL_CONTROLS,
        ),
        rebindable(
            &["rail_lever_zero"],
            "X",
            &[KeyCode::KeyX],
            keys::HELP_KEY_RAIL_ZERO,
            RAIL_CONTROLS,
        ),
        rebindable(
            &["rail_switch_toggle"],
            "G",
            &[KeyCode::KeyG],
            keys::HELP_KEY_RAIL_SWITCH,
            RAIL,
        ),
        rebindable(
            &[
                "rail_branch_left",
                "rail_branch_straight",
                "rail_branch_right",
            ],
            "← ↑ →",
            &[KeyCode::ArrowLeft, KeyCode::ArrowUp, KeyCode::ArrowRight],
            keys::HELP_KEY_TRAM,
            RAIL,
        ),
    ],
);
