//! Simulation clock (time, date, sun, pause), sound, clutch, weather and precipitation rows.

use super::super::{InputRow, Kind, Source};
use super::row;

/// Rows 30..43 of the input table, in table order.
pub(super) const ROWS: [InputRow; 13] = [
    // System values (`(L.S.x)`); `Timegap` and `GetTime` are set by `run_frame`.
    row("time", "s of day", Source::Clock, Kind::System, false, true),
    row("day", "1..31", Source::Clock, Kind::System, false, true),
    row("month", "1..12", Source::Clock, Kind::System, false, true),
    row("year", "year", Source::Clock, Kind::System, false, true),
    row(
        "dayofyear",
        "1..366",
        Source::Clock,
        Kind::System,
        false,
        true,
    ),
    row("sunalt", "deg", Source::Clock, Kind::System, false, true),
    row("pause", "0/1", Source::Clock, Kind::System, false, true),
    row(
        "nosound",
        "0/1",
        Source::Constant,
        Kind::System,
        false,
        false,
    ),
    row(
        "weather_temperature",
        "°C",
        Source::Weather,
        Kind::System,
        false,
        true,
    ),
    row(
        "weather_abshum",
        "g/m³",
        Source::Weather,
        Kind::System,
        false,
        true,
    ),
    row(
        "preciprate",
        "0..1",
        Source::Weather,
        Kind::System,
        false,
        true,
    ),
    row(
        "preciptype",
        "0 none/1 rain/2 snow",
        Source::Weather,
        Kind::System,
        false,
        true,
    ),
    row(
        "autoclutch",
        "0/1",
        Source::Planner,
        Kind::System,
        false,
        true,
    ),
];
