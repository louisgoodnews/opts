//! What a map says about its AI traffic, and the new and old model's group factors at noon.

use opts_core::formats::map::lists::{day, GroupDensity, UnschedGroup};
use opts_core::formats::map::{AiLists, DensityProfile, MapManifest};

pub(crate) const HOUR: f32 = 12.0;

/// What a map says about its AI traffic.
pub(crate) struct Traffic {
    pub(crate) manifest: MapManifest,
    pub(crate) lists: AiLists,
    pub(crate) vg: Vec<UnschedGroup>,
    pub(crate) dens: Vec<GroupDensity>,
}

/// The new model's groups with their factor, and the old model's factor sum.
pub(crate) fn group_weights(t: &Traffic) -> (Vec<(usize, f64)>, f64) {
    let road = DensityProfile {
        days: 0,
        points: t.manifest.traffic_density_road.clone(),
    };
    // New: every listed group present in the AI lists (else NormalCars / first as group 0).
    let mut groups: Vec<(usize, f64)> =
        t.vg.iter()
            .enumerate()
            .filter(|(_, g)| t.lists.group(&g.name).is_some())
            .map(|(i, g)| {
                let f = t
                    .dens
                    .iter()
                    .find(|d| d.name.eq_ignore_ascii_case(&g.name))
                    .map_or(road.at(HOUR), |d| d.density(day::WEEKDAY, HOUR));
                (i, f64::from(f))
            })
            .collect();
    // Old: groups with default ≥ 1, factor clamped, 1 without a curve.
    let mut old_sum: f64 =
        t.vg.iter()
            .filter(|g| g.default_density >= 1 && t.lists.group(&g.name).is_some())
            .map(|g| {
                f64::from(
                    t.dens
                        .iter()
                        .find(|d| d.name.eq_ignore_ascii_case(&g.name))
                        .map_or(1.0, |d| d.density(day::WEEKDAY, HOUR))
                        .clamp(0.0, 2.0),
                )
            })
            .sum();
    if groups.is_empty() && !t.lists.groups.is_empty() {
        groups.push((0, f64::from(road.at(HOUR))));
    }
    if old_sum == 0.0 && !t.lists.groups.is_empty() {
        old_sum = 1.0;
    }
    (groups, old_sum)
}
