//! What the unscheduled traffic aims for: the density plan of the session's hour and day.

use super::super::variants::{baseline_targets, group_lane_metres, time_factor};
use super::*;

/// What the unscheduled traffic aims for.
pub(super) struct DensityPlan {
    /// Vehicles in total.
    pub(super) target: usize,
    /// Per group: the target minus the vehicles alive.
    pub(super) deficits: Vec<f64>,
    /// Per group: the vehicles its lane metres and time factor ask for.
    pub(super) wanted: Vec<f64>,
    /// Per group: the time factor of the session's hour and day.
    pub(super) factors: Vec<f64>,
    /// Per group: the vehicles alive.
    pub(super) alive: Vec<usize>,
    /// The roads cannot hold the baseline.
    pub(super) capped: bool,
    pub(super) hour: f32,
    pub(super) day_bit: u8,
}

/// The density model: per group, the lane metres within the despawn disc weighted by the group's
/// density on each lane, times its time factor for the session's hour and day, per `L0`; the
/// baseline setting spread over the groups and limited to what the roads hold.
pub(super) fn density_plan(
    pool: &mut TrafficPool,
    state: &TrafficState,
    baseline: usize,
    centre: DVec2,
    env: Option<&opts_bevy::SessionEnvironment>,
) -> DensityPlan {
    let (hour, day_bit) = env.map_or((12.0, day::WEEKDAY), |e| (e.hour(), e.day_bit()));
    let factors: Vec<f64> = pool
        .groups
        .iter()
        .map(|g| time_factor(g, &pool.road_curve, day_bit, hour))
        .collect();
    let metres = group_lane_metres(
        state.world.graph(),
        centre,
        f64::from(DESPAWN_M),
        &pool.groups,
        &pool.defaults,
    );
    // Depot groups drive timetable runs only, never as unscheduled traffic.
    let metres: Vec<f64> = metres
        .iter()
        .zip(&pool.groups)
        .map(|(m, g)| if g.scheduled { 0.0 } else { *m })
        .collect();
    // The baseline setting spread over the groups, scaled by the map's factors for this hour and
    // day, limited to what the roads hold.
    let (targets, capped) = baseline_targets(&metres, &factors, baseline);
    let wanted: Vec<f64> = metres
        .iter()
        .zip(&factors)
        .map(|(m, f)| m * f / METRES_OF_LANE_PER_VEHICLE)
        .collect();
    if capped != pool.capped {
        pool.capped = capped;
        if capped {
            tracing::info!(
                baseline = baseline,
                room = ?targets.iter().sum::<f64>(),
                "the roads are full: every group scaled down in proportion"
            );
        } else {
            tracing::info!(baseline = baseline, "the roads hold the baseline again");
        }
    }
    let target = targets.iter().sum::<f64>().round() as usize;
    // Vehicles per group.
    let TrafficPool {
        agent_groups,
        groups,
        ..
    } = &mut *pool;
    agent_groups.retain(|id, _| state.world.agent(*id).is_some());
    let mut alive = vec![0usize; groups.len()];
    for &g in agent_groups.values() {
        if let Some(a) = alive.get_mut(g) {
            *a += 1;
        }
    }
    let deficits: Vec<f64> = targets
        .iter()
        .zip(&alive)
        .map(|(t, &a)| t - a as f64)
        .collect();
    DensityPlan {
        target,
        deficits,
        wanted,
        factors,
        alive,
        capped,
        hour,
        day_bit,
    }
}
