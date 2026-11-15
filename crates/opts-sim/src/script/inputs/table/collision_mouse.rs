//! Collision events and the mouse while a hot-spot is held.

use super::super::{InputRow, Kind, Source};
use super::row;

/// Rows 43..50 of the input table, in table order.
pub(super) const ROWS: [InputRow; 7] = [
    row(
        "coll_energy",
        "J",
        Source::Collision,
        Kind::System,
        false,
        false,
    ),
    row(
        "coll_pos_x",
        "m",
        Source::Collision,
        Kind::System,
        false,
        false,
    ),
    row(
        "coll_pos_y",
        "m",
        Source::Collision,
        Kind::System,
        false,
        false,
    ),
    row(
        "coll_pos_z",
        "m",
        Source::Collision,
        Kind::System,
        false,
        false,
    ),
    row(
        "wearlifespan",
        "0..1",
        Source::Environment,
        Kind::System,
        false,
        true,
    ),
    row("mouse_x", "px", Source::Mouse, Kind::System, false, true),
    row("mouse_y", "px", Source::Mouse, Kind::System, false, true),
];
