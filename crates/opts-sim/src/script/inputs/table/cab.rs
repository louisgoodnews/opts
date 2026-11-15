//! Cab and train state: seat, colour scheme, couplings, reverse, priority and clutch rows.

use super::super::{InputRow, Kind, Source};
use super::row;

/// Rows 23..30 of the input table, in table order.
pub(super) const ROWS: [InputRow; 7] = [
    row(
        "driver_seat_verttransl",
        "m",
        Source::Constant,
        Kind::Float,
        false,
        false,
    ),
    row(
        "colorscheme",
        "index",
        Source::Constant,
        Kind::Float,
        true,
        false,
    ),
    row(
        "train_frontcoupling",
        "0/1",
        Source::Rail,
        Kind::Float,
        false,
        false,
    ),
    row(
        "train_backcoupling",
        "0/1",
        Source::Rail,
        Kind::Float,
        false,
        false,
    ),
    row(
        "train_me_reverse",
        "0/1",
        Source::Rail,
        Kind::Float,
        false,
        false,
    ),
    row(
        "trafficpriority",
        "code",
        Source::Traffic,
        Kind::Float,
        true,
        false,
    ),
    row(
        "trafficprioritywarningneeded",
        "0/1",
        Source::Traffic,
        Kind::Float,
        false,
        false,
    ),
];
