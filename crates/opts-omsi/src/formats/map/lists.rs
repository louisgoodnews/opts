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

/// Parsed `ailists.cfg` (current `_2` syntax only).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AiLists {
    /// Groups in file order.
    pub groups: Vec<AiGroup>,
    /// Vehicle files of the depot groups (`[aigroup_depot_typgroup_2]`: the operator's own fleet,
    /// first line of each block), duplicates removed, in file order.
    pub depot_vehicles: Vec<String>,
}

impl AiLists {
    /// Parses `ailists.cfg`. Lines that do not look like `<file>[<TAB><weight>]` are skipped.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Self {
        let mut out = Self::default();
        for block in scan_blocks(&decode_bytes(bytes)) {
            if block.keyword == "aigroup_depot_typgroup_2" {
                if let Some(file) = block.parameters.first().map(|l| l.trim()) {
                    let lower = file.to_ascii_lowercase();
                    let known = [".bus", ".ovh", ".zug"].iter().any(|e| lower.ends_with(e));
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
mod tests {
    use super::*;

    #[test]
    fn ai_groups_with_and_without_weights() {
        let text = b"[aigroup_2]\r\nNormalCars\r\n\r\nvehicles\\Golf\\AI_Golf.bus\t7\r\nvehicles\\T3\\Van.ovh\r\n[end]\r\n[aigroup_depot]\r\nHof\r\nSpandau\r\n[end]\r\n";
        let lists = AiLists::parse(text);
        assert_eq!(lists.groups.len(), 1);
        let g = lists.group("normalcars").expect("case-insensitive");
        assert_eq!(g.entries.len(), 2);
        assert_eq!(g.entries[0].weight, Some(7.0));
        assert_eq!(g.entries[1].weight, None);
        assert_eq!(g.total_weight(), 8.0);
    }

    #[test]
    fn depot_groups_keep_their_vehicle_file_once() {
        let text = b"[aigroup_depot]\r\nEVAG|NF2\r\nGladbeck\r\n\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\Tram_NF6D\\NF6D_KI.bus\r\n1\t\tEVAG\r\n[end]\r\n[aigroup_depot_typgroup_2]\r\nvehicles\\Tram_NF6D\\nf6d_ki.bus\r\n2\t\tEVAG\r\n[end]\r\n";
        let lists = AiLists::parse(text);
        assert!(lists.groups.is_empty());
        assert_eq!(lists.depot_vehicles, ["vehicles\\Tram_NF6D\\NF6D_KI.bus"]);
    }

    #[test]
    fn unsched_groups() {
        let g = parse_unsched_vehgroups(b"[group]\nNormalCars\n1\n\n[group]\nBicycles\n0\n");
        assert_eq!(g.len(), 2);
        assert_eq!((g[1].name.as_str(), g[1].default_density), ("Bicycles", 0));
    }

    #[test]
    fn density_curve_with_day_restriction() {
        let text = b"[group]\nNormalCars\n0.6\n\n[set_day_of_week]\n0\n\n[trafficdensity]\n0.000\n0.100\n[trafficdensity]\n4.000\n0.010\n\n[set_day_of_week]\n1\n\n[trafficdensity]\n0.000\n0.100\n[trafficdensity]\n6.000\n1.000\n";
        let d = parse_unsched_trafficdens(text);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].profiles.len(), 2);
        // All-days curve: 0.1 → 0.01 over four hours, times the 0.6 factor.
        assert!((d[0].density(day::SUNDAY, 2.0) - 0.6 * 0.055).abs() < 1e-5);
        // Weekday curve rises to 1.0 at 06:00.
        assert!((d[0].density(day::WEEKDAY, 3.0) - 0.6 * 0.55).abs() < 1e-5);
        assert!((d[0].density(day::WEEKDAY, 12.0) - 0.6).abs() < 1e-5);
    }

    /// Hours wrap at midnight; weekend curves are picked by the day bit.
    #[test]
    fn hours_wrap_and_weekends_have_their_own_curve() {
        let text = b"[group]\nNormalCars\n1\n\n[set_day_of_week]\n1\n[trafficdensity]\n0\n0.2\n[trafficdensity]\n12\n1.4\n[trafficdensity]\n24\n0.2\n\n[set_day_of_week]\n6\n[trafficdensity]\n0\n0.1\n[trafficdensity]\n24\n0.5\n";
        let d = &parse_unsched_trafficdens(text)[0];
        let wd = |h| d.density(day::WEEKDAY, h);
        assert!((wd(25.0) - wd(1.0)).abs() < 1e-6 && (wd(-1.0) - wd(23.0)).abs() < 1e-6);
        assert!((wd(36.0) - 1.4).abs() < 1e-6);
        assert!((d.density(day::SATURDAY, 12.0) - 0.3).abs() < 1e-6);
        assert!((d.density(day::SUNDAY, 36.0) - 0.3).abs() < 1e-6);
        // Values above 1 are kept.
        assert!(wd(12.0) > 1.0);
    }

    #[test]
    fn empty_profile_means_baseline() {
        let g = GroupDensity {
            name: "x".into(),
            factor: 2.0,
            profiles: Vec::new(),
        };
        assert_eq!(g.density(day::WEEKDAY, 8.0), 2.0);
    }
}
