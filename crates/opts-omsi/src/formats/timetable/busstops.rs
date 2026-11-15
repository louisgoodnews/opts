//! `Busstops.cfg`: the stop list of a map's timetables.

use super::text::{at, keyword, lines_of, num};

/// A stop of the map's timetable stop list.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtBusStop {
    /// Stop name, as the hof's stop idents spell it (empty for an unnamed platform).
    pub name: String,
    /// Second value of the record (350 distinct values over 5,506 stops of 46 maps: a stop
    /// group or area number ❓).
    pub group: i32,
    /// Station id, the number a trip file's `[station_typ2]` names.
    pub id: u32,
    /// Standing time at the stop (s ❓: 0, 1, 2, 3, 5 and 10 are the common values).
    pub dwell_s: f32,
    /// Fifth value (0 for 5,505 of 5,506 stops ❓).
    pub flag: i32,
    /// Sixth value: a `Chrono.cfg` path in one record, empty otherwise ❓.
    pub chrono: String,
}

/// Parses `Busstops.cfg`.
#[must_use]
pub fn parse_busstops(bytes: &[u8]) -> Vec<TtBusStop> {
    let l = lines_of(bytes);
    let mut out = Vec::new();
    let mut i = 0;
    while i < l.len() {
        if keyword(&l[i]).as_deref() == Some("busstop") {
            out.push(TtBusStop {
                name: at(&l, i + 1).to_string(),
                group: num(at(&l, i + 2)),
                id: num(at(&l, i + 3)),
                dwell_s: num(at(&l, i + 4)),
                flag: num(at(&l, i + 5)),
                chrono: at(&l, i + 6).to_string(),
            });
            i += 7;
        } else {
            i += 1;
        }
    }
    out
}
