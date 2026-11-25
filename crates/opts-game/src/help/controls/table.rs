//! The bindings table's helpers: the constructors of a [`Binding`], the source files the rows name,
//! and [`GROUPS`], the groups in the order Help prints them.

use bevy::prelude::KeyCode;

use super::driving::{DRIVING_GROUP, RAIL_GROUP};
use super::panels::{CAMERA_GROUP, MAP_GROUP, MENUS_GROUP};
use super::{Binding, Group};

/// Shorthand for [`Binding`] rows.
pub(super) const fn bind(
    shown: &'static str,
    keys: &'static [KeyCode],
    action: &'static str,
    source: &'static str,
) -> Binding {
    Binding {
        shown,
        keys,
        action,
        source,
        ids: &[],
    }
}

/// A [`Binding`] whose keys the player can change.
pub(super) const fn rebindable(
    ids: &'static [&'static str],
    shown: &'static str,
    keys: &'static [KeyCode],
    action: &'static str,
    source: &'static str,
) -> Binding {
    Binding {
        shown,
        keys,
        action,
        source,
        ids,
    }
}

pub(super) const SIM: &str = "crates/opts-bevy/src/vehicle_sim/input.rs";
pub(super) const RAIL_TRACK: &str = "crates/opts-world-load/src/rail/player_track.rs";
pub(super) const RAIL_CONTROLS: &str = "crates/opts-bevy/src/rail_controls.rs";
pub(super) const CAMERA: &str = "crates/opts-bevy/src/camera/input.rs";
pub(super) const VIRTUAL_MIRRORS: &str = "crates/opts-bevy/src/mirrors/virtual_view.rs";
pub(super) const EGO: &str = "crates/opts-bevy/src/camera/mode/ego.rs";
pub(super) const PAUSE: &str = "crates/opts-game/src/pause/input.rs";
pub(super) const SIM_PAUSE: &str = "crates/opts-game/src/in_game/pause_toggle.rs";
pub(super) const HELP: &str = "crates/opts-game/src/help/mod.rs";
pub(super) const THEME: &str = "crates/opts-game/src/shell/footer.rs";
pub(super) const DEV: &str = "crates/opts-game/src/in_game/omsi_world/overlay/panel.rs";
pub(super) const MINIMAP: &str = "crates/opts-game/src/hud/minimap/input.rs";
pub(super) const FULLMAP: &str = "crates/opts-game/src/hud/fullmap/open.rs";
pub(super) const DOOR_CAMERA: &str = "crates/opts-bevy/src/door_camera.rs";
pub(super) const SESSION: &str = "crates/opts-game/src/session_vehicles/mod.rs";
pub(super) const QUICK_SAVE: &str = "crates/opts-game/src/save/quick.rs";
pub(super) const ZONE_OVERLAY: &str =
    "crates/opts-game/src/in_game/omsi_world/spatial_zone_overlay.rs";
pub(super) const CAB_SWITCH: &str = "crates/opts-bevy/src/cab_switch.rs";
pub(super) const LANE_OVERLAY: &str =
    "crates/opts-world-load/src/traffic/lane_overlay.rs";

/// The single registry of the keyboard bindings that the Help page prints. The input systems keep
/// reading keys directly (they are not driven by this table); the test in this module reads their
/// source files and fails when a listed key is no longer used there. Add a row here when an input
/// system gets a new key, and name the row's file in the system's doc comment.
pub const GROUPS: &[Group] = &[
    DRIVING_GROUP,
    RAIL_GROUP,
    CAMERA_GROUP,
    MAP_GROUP,
    MENUS_GROUP,
];
