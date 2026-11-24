//! `StnLinks.cfg`: the links between stations along the road or track network, with their pieces.

use super::text::{at, keyword, lines_of, num};

/// One piece of path of a station link (`[StnLink_entry]`: seven values).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StnLinkEntry {
    /// First value: the id of a path or spline ❓ (an id repeats along a link when the path is
    /// cut into pieces).
    pub id: u32,
    /// Second and third values: small integers (0 to 196) ❓.
    pub b: i32,
    /// See `b`.
    pub c: i32,
    /// Fourth value: the length of the piece in metres; the pieces of a link add up to the
    /// link's `length_m` (median ratio 1.015 over 326 links of Berlin-Spandau).
    pub length_m: f32,
    /// Fifth value: `-1` in all 4,055 entries of Berlin-Spandau ❓.
    pub d: i32,
    /// Sixth value: `0` in all of them ❓.
    pub e: i32,
    /// Seventh value: `0` (`1` in 16 of 4,055) ❓.
    pub f: i32,
}

/// A link between two stations along the road or track network (`[StnLink]`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StnLink {
    /// Name of the first station, from the comment line `A ==> B` above the record.
    pub from_name: String,
    /// Name of the second station.
    pub to_name: String,
    /// First value: the length of the link in metres.
    pub length_m: f32,
    /// Station id of the first station (a `Busstops.cfg` id; the second value).
    pub from_id: u32,
    /// Station id of the second station (third value).
    pub to_id: u32,
    /// Fourth value: a number belonging to the first station (1.13 to 4.6 on the corpus; the same
    /// station has the same value in every link it starts or ends ❓: a lateral offset or a
    /// height).
    pub from_a: f32,
    /// Fifth value: the same for the second station.
    pub to_a: f32,
    /// Sixth value: a second number of the first station (7 to 87 ❓: a position along the
    /// path).
    pub from_b: f32,
    /// Seventh value: the same for the second station.
    pub to_b: f32,
    /// Eighth value: `0` in 321 of 326 links of Berlin-Spandau ❓.
    pub flag: i32,
    /// Ninth value: the index of the last entry (the entries number one more).
    pub last_entry: i32,
    /// The pieces of path.
    pub entries: Vec<StnLinkEntry>,
}

/// Parses `StnLinks.cfg`.
#[must_use]
pub fn parse_stn_links(bytes: &[u8]) -> Vec<StnLink> {
    let l = lines_of(bytes);
    let mut out: Vec<StnLink> = Vec::new();
    let mut i = 0;
    while i < l.len() {
        let Some(k) = keyword(&l[i]) else {
            i += 1;
            continue;
        };
        match k.as_str() {
            "stnlink" => {
                // The names are in the comment line `A ==> B` a line or two above.
                let (from_name, to_name) = (i.saturating_sub(3)..i)
                    .rev()
                    .find_map(|j| l[j].split_once("==>"))
                    .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
                    .unwrap_or_default();
                out.push(StnLink {
                    from_name,
                    to_name,
                    length_m: num(at(&l, i + 1)),
                    from_id: num::<f64>(at(&l, i + 2)) as u32,
                    to_id: num::<f64>(at(&l, i + 3)) as u32,
                    from_a: num(at(&l, i + 4)),
                    to_a: num(at(&l, i + 5)),
                    from_b: num(at(&l, i + 6)),
                    to_b: num(at(&l, i + 7)),
                    flag: num::<f64>(at(&l, i + 8)) as i32,
                    last_entry: num::<f64>(at(&l, i + 9)) as i32,
                    entries: Vec::new(),
                });
                i += 10;
            }
            "stnlink_entry" => {
                if let Some(link) = out.last_mut() {
                    link.entries.push(StnLinkEntry {
                        id: num::<f64>(at(&l, i + 1)) as u32,
                        b: num::<f64>(at(&l, i + 2)) as i32,
                        c: num::<f64>(at(&l, i + 3)) as i32,
                        length_m: num(at(&l, i + 4)),
                        d: num::<f64>(at(&l, i + 5)) as i32,
                        e: num::<f64>(at(&l, i + 6)) as i32,
                        f: num::<f64>(at(&l, i + 7)) as i32,
                    });
                }
                i += 8;
            }
            _ => i += 1,
        }
    }
    out
}
