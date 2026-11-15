//! Traffic, passengers and the environment: brightness, road, dirt, wear, precipitation.

use super::super::{InputRow, Kind, Source};
use super::row;

/// Rows 14..23 of the input table, in table order.
pub(super) const ROWS: [InputRow; 9] = [
    row(
        "schedule_active",
        "0/1",
        Source::Timetable,
        Kind::Float,
        false,
        true,
    ),
    row(
        "humans_count",
        "count",
        Source::Passengers,
        Kind::Float,
        false,
        true,
    ),
    row(
        "envir_brightness",
        "0..1",
        Source::Environment,
        Kind::Float,
        false,
        true,
    ),
    row(
        "streetcond",
        "0..1",
        Source::Environment,
        Kind::Float,
        false,
        true,
    ),
    row(
        "dirtrate",
        "0..1",
        Source::Environment,
        Kind::Float,
        false,
        true,
    ),
    row(
        "wearlifespan",
        "0..1",
        Source::Environment,
        Kind::Float,
        false,
        true,
    ),
    row(
        "preciprate",
        "0..1",
        Source::Weather,
        Kind::Float,
        false,
        true,
    ),
    row(
        "preciptype",
        "0 none/1 rain/2 snow",
        Source::Weather,
        Kind::Float,
        false,
        true,
    ),
    row("relrange", "?", Source::Constant, Kind::Float, false, false),
];
