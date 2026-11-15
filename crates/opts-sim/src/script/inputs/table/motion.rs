//! Vehicle drive: speeds, wheels, accelerations, odometer and the driver pedals.

use super::super::{InputRow, Kind, Source};
use super::row;

/// Rows 0..14 of the input table, in table order.
pub(super) const ROWS: [InputRow; 14] = [
    row(
        "velocity",
        "km/h",
        Source::Physics,
        Kind::Float,
        false,
        true,
    ),
    row(
        "velocity_ground",
        "km/h",
        Source::Physics,
        Kind::Float,
        false,
        true,
    ),
    row("n_wheel", "rpm", Source::Physics, Kind::Float, false, true),
    row(
        "a_trans_x",
        "m/s²",
        Source::Physics,
        Kind::Float,
        false,
        true,
    ),
    row(
        "a_trans_y",
        "m/s²",
        Source::Physics,
        Kind::Float,
        false,
        true,
    ),
    row(
        "a_trans_z",
        "m/s²",
        Source::Physics,
        Kind::Float,
        false,
        false,
    ),
    row(
        "kmcounter_km",
        "km",
        Source::Physics,
        Kind::Float,
        false,
        true,
    ),
    row(
        "kmcounter_m",
        "m",
        Source::Physics,
        Kind::Float,
        false,
        true,
    ),
    row("throttle", "0..1", Source::Planner, Kind::Float, true, true),
    row("brake", "0..1", Source::Planner, Kind::Float, true, true),
    row("clutch", "0..1", Source::Planner, Kind::Float, true, true),
    row("ai", "0/1", Source::Planner, Kind::Float, true, true),
    row(
        "ai_engine",
        "-1/0/1",
        Source::Planner,
        Kind::Float,
        true,
        true,
    ),
    row(
        "ai_target_index",
        "hof row",
        Source::Timetable,
        Kind::Float,
        true,
        true,
    ),
];
