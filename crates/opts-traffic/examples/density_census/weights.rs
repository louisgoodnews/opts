//! `S`: the lane length inside the disc around an entry point, weighted by density and group factor.

use glam::DVec2;
use opts_traffic::LaneGraph;

/// Radius (m) of the disc around the entry point.
const DISC_M: f64 = 320.0;

/// `S`: the lane length inside the disc around `centre`, weighted by density and group factor.
pub(crate) fn weighted_length(
    graph: &LaneGraph,
    centre: DVec2,
    groups: &[(usize, f64)],
    defaults: &[u32],
) -> f64 {
    let mut s = 0.0;
    for lane in graph.lane_ids().filter_map(|l| graph.lane(l)) {
        let len = lane.length();
        let inside = (0..=8)
            .filter(|&i| {
                lane.geometry
                    .point_at(len * f64::from(i) / 8.0)
                    .distance(centre)
                    <= DISC_M
            })
            .count();
        let metres = len * inside as f64 / 9.0;
        for &(g, f) in groups {
            s += metres * lane.rules.density.for_group(g, defaults) * f;
        }
    }
    s
}
