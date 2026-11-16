//! The table of host input rows: the script variable each host input feeds.

//! The rows are split by section (`table/*.rs`); [`INPUT_TABLE`] keeps the table order.

use super::*;

mod cab;
mod clock_weather;
mod collision_mouse;
mod motion;
mod scene;

const fn row(
    name: &'static str,
    unit: &'static str,
    source: Source,
    kind: Kind,
    shared: bool,
    live: bool,
) -> InputRow {
    InputRow {
        name,
        unit,
        source,
        kind,
        shared,
        live,
    }
}

/// The sections in table order.
const SECTIONS: &[&[InputRow]] = &[
    &motion::ROWS,
    &scene::ROWS,
    &cab::ROWS,
    &clock_weather::ROWS,
    &collision_mouse::ROWS,
];

/// Scalar inputs. Wheel and door families are in [`WHEEL_FAMILIES`] and [`DOOR_FAMILIES`].
pub const INPUT_TABLE: &[InputRow] = &concat::<51>(SECTIONS);

/// Joins the sections into one table; panics at compile time if the length is not `N`.
const fn concat<const N: usize>(sections: &[&[InputRow]]) -> [InputRow; N] {
    let mut out = [row("", "", Source::Constant, Kind::Float, false, false); N];
    let mut i = 0;
    let mut s = 0;
    while s < sections.len() {
        let section = sections[s];
        let mut j = 0;
        while j < section.len() {
            out[i] = section[j];
            i += 1;
            j += 1;
        }
        s += 1;
    }
    assert!(i == N, "input table length mismatch");
    out
}
