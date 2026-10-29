//! Unscheduled traffic density profiles.

use crate::formats::text::{decode_bytes, scan_blocks};

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
