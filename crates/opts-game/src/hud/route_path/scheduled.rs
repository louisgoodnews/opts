//! The scheduled vehicles for the maps: where each one of the timetable is.

use super::*;

/// Where the scheduled vehicles are, for the maps.
#[derive(Resource, Default, Debug, Clone)]
pub struct ScheduledVehicles {
    /// World position (east, north) and compass heading.
    pub poses: Vec<(DVec2, f32)>,
    since: f32,
}

pub(super) fn update_scheduled_vehicles(
    time: Res<Time<Real>>,
    mut out: ResMut<ScheduledVehicles>,
    traffic: Option<Res<TrafficState>>,
    sched: Option<Res<crate::in_game::omsi_world::ScheduledTraffic>>,
    trains: Query<&GlobalTransform, With<crate::in_game::omsi_world::ScheduledTrain>>,
) {
    out.since += time.delta_secs();
    if out.since < 0.2 {
        return;
    }
    out.since = 0.0;
    out.poses.clear();
    if let (Some(t), Some(s)) = (traffic, sched) {
        t.poses_where(|id| s.run_of(id).is_some(), &mut out.poses);
    }
    for gt in &trains {
        let (p, h) = crate::hud::minimap::pose_of(gt);
        out.poses.push((p, h));
    }
}
