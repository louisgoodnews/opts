//! AI traffic lists of a map folder (`docs/src/formats/map/lists.md`): `ailists.cfg`,
//! `unsched_vehgroups.txt` and `unsched_trafficdens.txt`.
//!
//! These files say *which* vehicles may appear as AI traffic and *how dense* that traffic is over
//! the day. They do not say where vehicles drive: that comes from the `[path]` lanes of splines and
//! junction objects (`opts-traffic`). The depot groups' vehicle files are kept in
//! [`AiLists::depot_vehicles`]; the legacy `[ailist]` syntax is skipped for now.

use crate::formats::text::{decode_bytes, scan_blocks};

/// One vehicle of an AI group.
#[derive(Debug, Clone, PartialEq)]
pub struct AiEntry {
    /// Vehicle file as written (`.bus`, `.ovh` or `.zug`; backslashes, relative to the OMSI root).
    pub vehicle: String,
    /// Relative spawn frequency; `None` when the line carries no weight (the default share ❓).
    pub weight: Option<f32>,
}

/// A named set of AI vehicles (`[aigroup_2]` … `[end]`).
#[derive(Debug, Clone, PartialEq)]
pub struct AiGroup {
    /// Group name (`NormalCars`, `Trucks`, …); referenced by name from `unsched_*.txt`.
    pub name: String,
    /// Vehicles in file order.
    pub entries: Vec<AiEntry>,
}

impl AiGroup {
    /// Sum of the weights, counting a missing weight as `1`.
    #[must_use]
    pub fn total_weight(&self) -> f32 {
        self.entries.iter().map(|e| e.weight.unwrap_or(1.0)).sum()
    }
}

/// One vehicle of a depot's fleet: a line `<number><TAB><registration><TAB><repaint>` after the
/// vehicle file of an `[aigroup_depot_typgroup_2]` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetEntry {
    /// Fleet number (`101`).
    pub number: String,
    /// Registration plate (`WIN-SW 101`).
    pub plate: String,
    /// The repaint it wears (`Winsenburg`); empty when the line names none.
    pub repaint: String,
    /// First day (`yyyymmdd`) the vehicle is in service, if the line says.
    pub from: Option<u32>,
    /// Last day (`yyyymmdd`) the vehicle is in service, if the line says.
    pub to: Option<u32>,
}

impl FleetEntry {
    /// Whether the vehicle is in service on `date` (`yyyymmdd`).
    #[must_use]
    pub fn in_service(&self, date: u32) -> bool {
        self.from.is_none_or(|f| date >= f) && self.to.is_none_or(|t| date <= t)
    }
}

/// A depot group of `ailists.cfg` (`[aigroup_depot]`): the vehicles an operator's depot sends
/// out. A tour of the timetable names the group whose vehicles drive it.
#[derive(Debug, Clone, PartialEq)]
pub struct DepotGroup {
    /// Group name (`AVG Solowagen (Kranenburg)`), the `vehicle_group` of a timetable tour.
    pub name: String,
    /// The depot it belongs to (second line), empty when absent.
    pub depot: String,
    /// Vehicle files of its `[aigroup_depot_typgroup_2]` blocks, in file order (with repeats,
    /// which weigh the choice).
    pub vehicles: Vec<String>,
    /// The fleet lines of the type groups: vehicle file and the entry that follows it, in file
    /// order.
    pub fleet: Vec<(String, FleetEntry)>,
}

/// Parsed `ailists.cfg` (current `_2` syntax only).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AiLists {
    /// Groups in file order.
    pub groups: Vec<AiGroup>,
    /// Vehicle files of the depot groups (`[aigroup_depot_typgroup_2]`: the operator's own fleet,
    /// first line of each block), duplicates removed, in file order.
    pub depot_vehicles: Vec<String>,
    /// The depot groups with their vehicles: the `[aigroup_depot_typgroup_2]` blocks after an
    /// `[aigroup_depot]` block belong to it.
    pub depot_groups: Vec<DepotGroup>,
}

impl AiLists {
    /// Parses `ailists.cfg`. Lines that do not look like `<file>[<TAB><weight>]` are skipped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        for block in scan_blocks(&decode_bytes(bytes)) {
            if block.keyword == "aigroup_depot" {
                out.depot_groups.push(DepotGroup {
                    name: block
                        .parameters
                        .first()
                        .map_or_else(String::new, |l| l.trim().to_owned()),
                    depot: block
                        .parameters
                        .get(1)
                        .map_or_else(String::new, |l| l.trim().to_owned()),
                    vehicles: Vec::new(),
                    fleet: Vec::new(),
                });
                continue;
            }
            if block.keyword == "aigroup_depot_typgroup_2" {
                if let Some(file) = block.parameters.first().map(|l| l.trim()) {
                    let lower = file.to_ascii_lowercase();
                    let known = [".bus", ".ovh", ".zug"].iter().any(|e| lower.ends_with(e));
                    if known {
                        if let Some(g) = out.depot_groups.last_mut() {
                            g.vehicles.push(file.to_owned());
                            for line in block.parameters.iter().skip(1) {
                                // The plate may be empty (`2708<TAB><TAB>repaint`): keep the columns.
                                let cols: Vec<&str> = line.split('\t').map(str::trim).collect();
                                let Some(number) = cols.first().filter(|c| !c.is_empty()) else {
                                    continue;
                                };
                                let col = |i: usize| cols.get(i).copied().filter(|c| !c.is_empty());
                                let (plate, repaint) = (col(1), col(2));
                                let day = |i: usize| col(i).and_then(|c| c.parse::<u32>().ok());
                                g.fleet.push((
                                    file.to_owned(),
                                    FleetEntry {
                                        number: (*number).to_owned(),
                                        plate: plate.unwrap_or_default().to_owned(),
                                        repaint: repaint.unwrap_or_default().to_owned(),
                                        from: day(3),
                                        to: day(4),
                                    },
                                ));
                            }
                        }
                    }
                    if known
                        && !out
                            .depot_vehicles
                            .iter()
                            .any(|v| v.eq_ignore_ascii_case(file))
                    {
                        out.depot_vehicles.push(file.to_owned());
                    }
                }
                continue;
            }
            if block.keyword != "aigroup_2" {
                continue;
            }
            let mut lines = block.parameters.iter();
            let Some(name) = lines.next() else { continue };
            let entries = lines
                .filter_map(|l| {
                    let (file, weight) = match l.rsplit_once('\t') {
                        Some((f, w)) => (f.trim(), w.trim().parse::<f32>().ok()),
                        None => (l.trim(), None),
                    };
                    let lower = file.to_ascii_lowercase();
                    (lower.ends_with(".bus") || lower.ends_with(".ovh") || lower.ends_with(".zug"))
                        .then(|| AiEntry {
                            vehicle: file.to_owned(),
                            weight,
                        })
                })
                .collect();
            out.groups.push(AiGroup {
                name: name.trim().to_owned(),
                entries,
            });
        }
        out
    }

    /// The depot group called `name` (case-insensitive).
    #[must_use]
    pub fn depot_group(&self, name: &str) -> Option<&DepotGroup> {
        self.depot_groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(name))
    }

    /// The group called `name` (case-insensitive).
    #[must_use]
    pub fn group(&self, name: &str) -> Option<&AiGroup> {
        self.groups
            .iter()
            .find(|g| g.name.eq_ignore_ascii_case(name))
    }
}

/// One entry of `unsched_vehgroups.txt`.
#[derive(Debug, Clone, PartialEq)]
pub struct UnschedGroup {
    /// AI group name.
    pub name: String,
    /// Default density class: `0` no traffic on a path unless freed, `1` normal, higher values
    /// select further density classes.
    pub default_density: u32,
}

/// Parses `unsched_vehgroups.txt` (`[group]`: name, default density).
#[must_use]
pub fn parse_unsched_vehgroups(bytes: &[u8]) -> Vec<UnschedGroup> {
    scan_blocks(&decode_bytes(bytes))
        .into_iter()
        .filter(|b| b.keyword == "group")
        .filter_map(|b| {
            let name = b.parameters.first()?.trim().to_owned();
            let default_density = b.parameters.get(1)?.trim().parse().ok()?;
            Some(UnschedGroup {
                name,
                default_density,
            })
        })
        .collect()
}

/// Day-of-week bits of `[set_day_of_week]`: `0` = every day, otherwise a sum of these.
pub mod day {
    /// Monday to Friday.
    pub const WEEKDAY: u8 = 1;
    /// Saturday.
    pub const SATURDAY: u8 = 2;
    /// Sunday (and public holidays ❓).
    pub const SUNDAY: u8 = 4;
}

/// A piecewise-linear density curve over the hours of a day.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DensityProfile {
    /// Days the curve applies to (`0` = all, else a sum of [`day`] bits).
    pub days: u8,
    /// `(hour 0..=24, density)` points in file order.
    pub points: Vec<(f32, f32)>,
}

impl DensityProfile {
    /// Density at `hour` (linear between points, constant beyond the first and last point;
    /// `1.0` for a profile without points). The hour wraps at midnight (`25` is `1`, `-1` is
    /// `23`), so a session clock running past 24 h keeps following the curve.
    #[must_use]
    pub fn at(&self, hour: f32) -> f32 {
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return 1.0;
        };
        let hour = if hour.is_finite() {
            hour.rem_euclid(24.0)
        } else {
            0.0
        };
        if hour <= first.0 {
            return first.1;
        }
        for w in self.points.windows(2) {
            let ((h0, d0), (h1, d1)) = (w[0], w[1]);
            if hour <= h1 {
                return if h1 > h0 {
                    d0 + (d1 - d0) * (hour - h0) / (h1 - h0)
                } else {
                    d1
                };
            }
        }
        last.1
    }

    /// Whether the profile applies on a day described by one [`day`] bit.
    #[must_use]
    pub fn covers(&self, day_bit: u8) -> bool {
        self.days == 0 || self.days & day_bit != 0
    }
}

/// Density settings of one AI group (`[group]` in `unsched_trafficdens.txt`).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDensity {
    /// AI group name.
    pub name: String,
    /// Factor multiplied with the map's density functions.
    pub factor: f32,
    /// Curves in file order, each with the days it applies to.
    pub profiles: Vec<DensityProfile>,
}

impl GroupDensity {
    /// Density factor at `hour` on a day with the given [`day`] bit: `factor` times the first
    /// profile that covers the day, preferring day-specific profiles over the all-days one.
    #[must_use]
    pub fn density(&self, day_bit: u8, hour: f32) -> f32 {
        let specific = self
            .profiles
            .iter()
            .find(|p| p.days != 0 && p.covers(day_bit));
        let any = self.profiles.iter().find(|p| p.days == 0);
        let profile = specific.or(any);
        self.factor * profile.map_or(1.0, |p| p.at(hour))
    }
}

/// Parses `unsched_trafficdens.txt`.
#[must_use]
pub fn parse_unsched_trafficdens(bytes: &[u8]) -> Vec<GroupDensity> {
    let mut out: Vec<GroupDensity> = Vec::new();
    let mut days = 0u8;
    for block in scan_blocks(&decode_bytes(bytes)) {
        match block.keyword.as_str() {
            "group" => {
                let (Some(name), Some(factor)) = (
                    block.parameters.first(),
                    block
                        .parameters
                        .get(1)
                        .and_then(|f| f.trim().replace(',', ".").parse::<f32>().ok()),
                ) else {
                    continue;
                };
                days = 0;
                out.push(GroupDensity {
                    name: name.trim().to_owned(),
                    factor,
                    profiles: Vec::new(),
                });
            }
            "set_day_of_week" => {
                days = block
                    .parameters
                    .first()
                    .and_then(|d| d.trim().parse().ok())
                    .unwrap_or(0);
            }
            "trafficdensity" => {
                let (Some(group), [h, d, ..]) = (out.last_mut(), block.parameters.as_slice())
                else {
                    continue;
                };
                let (Ok(h), Ok(d)) = (
                    h.trim().replace(',', ".").parse::<f32>(),
                    d.trim().replace(',', ".").parse::<f32>(),
                ) else {
                    continue;
                };
                if group.profiles.last().map(|p| p.days) != Some(days) {
                    group.profiles.push(DensityProfile {
                        days,
                        points: Vec::new(),
                    });
                }
                if let Some(p) = group.profiles.last_mut() {
                    p.points.push((h, d));
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests;
