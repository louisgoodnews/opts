//! The lane budget of the traffic: the time factor of a group, the lane metres each group has
//! around the camera, and the vehicles wanted per group.

use super::super::*;

/// Time factor of a group: its `unsched_trafficdens.txt` factor times its curve for the hour and
/// day (`[set_day_of_week]`), or the map's `[trafficdensity_road]` curve when the group has no
/// entry there. Unclamped: values above 1 mean denser traffic.
pub(in crate::in_game) fn time_factor(
    group: &PoolGroup,
    road_curve: &DensityProfile,
    day_bit: u8,
    hour: f32,
) -> f64 {
    f64::from(match &group.density {
        Some(d) => d.density(day_bit, hour),
        None => road_curve.at(hour),
    })
    .max(0.0)
}

/// Lane metres inside the disc of `radius` around `centre`, weighted by each pool group's
/// density on the lane: `Σ_lanes length_inside × D(lane, group)`.
pub(in crate::in_game) fn group_lane_metres(
    graph: &LaneGraph,
    centre: DVec2,
    radius: f64,
    groups: &[PoolGroup],
    defaults: &[u32],
) -> Vec<f64> {
    const SAMPLES: usize = 8;
    let mut out = vec![0.0; groups.len()];
    for lane in graph.lane_ids().filter_map(|l| graph.lane(l)) {
        if lane.kind != LaneKind::Road {
            continue;
        }
        let len = lane.length();
        let inside = (0..=SAMPLES)
            .filter(|&i| {
                lane.geometry
                    .point_at(len * i as f64 / SAMPLES as f64)
                    .distance(centre)
                    <= radius
            })
            .count();
        if inside == 0 {
            continue;
        }
        let metres = len * inside as f64 / (SAMPLES + 1) as f64;
        for (g, group) in groups.iter().enumerate() {
            out[g] += metres * lane.rules.density.for_group(group.density_group, defaults);
        }
    }
    out
}

/// The vehicles wanted per group: the baseline spread over the groups by their density-weighted
/// lane metres, each scaled by its time factor (so the total is the baseline times the average
/// factor), limited to what the roads hold ([`MIN_LANE_SPACING_M`]). The flag says the limit
/// bound.
pub(in super::super) fn baseline_targets(
    metres: &[f64],
    factors: &[f64],
    baseline: usize,
) -> (Vec<f64>, bool) {
    let total: f64 = metres.iter().sum();
    if total <= 0.0 || baseline == 0 {
        return (vec![0.0; metres.len()], false);
    }
    let wanted: Vec<f64> = metres
        .iter()
        .zip(factors)
        .map(|(m, f)| baseline as f64 * m / total * f)
        .collect();
    let room = metres.iter().zip(factors).map(|(m, f)| m * f).sum::<f64>() / MIN_LANE_SPACING_M;
    group_targets(&wanted, room.floor().max(0.0) as usize)
}
