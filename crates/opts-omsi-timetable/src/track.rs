//! `.ttr` rail track files: the track entries (rail is M-0098).

use super::text::{at, keyword, lines_of, num};

/// One entry of a rail track file (`[track_entry]`: six values after the keyword).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TtTrackEntry {
    /// First value (a track or path id ❓).
    pub id: u32,
    /// Second to sixth values (0, 92, 41, a length in metres and 0 in the sample); their
    /// meaning is not known ❓.
    pub values: [f32; 5],
}

/// Parses a `.ttr` track file (bus maps ignore them; rail is M-0098).
#[must_use]
pub fn parse_track(bytes: &[u8]) -> Vec<TtTrackEntry> {
    let l = lines_of(bytes);
    let mut out = Vec::new();
    let mut i = 0;
    while i < l.len() {
        if keyword(&l[i]).as_deref() == Some("track_entry") {
            out.push(TtTrackEntry {
                id: num(at(&l, i + 1)),
                values: [
                    num(at(&l, i + 2)),
                    num(at(&l, i + 3)),
                    num(at(&l, i + 4)),
                    num(at(&l, i + 5)),
                    num(at(&l, i + 6)),
                ],
            });
            i += 7;
        } else {
            i += 1;
        }
    }
    out
}
