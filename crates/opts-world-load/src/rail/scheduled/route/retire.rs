//! Retirement of a scheduled train that has reached its last stop.

use super::super::*;

use super::RETIRE_FAR_M;

/// Whether a scheduled train that has reached its last stop may be removed now: its time has
/// come and the camera is far away.
pub(in super::super::super) fn retired(
    t: &AiTrain,
    net: &PedestrianNet,
    clock: f64,
    cam: Option<Vec3>,
) -> bool {
    let Some(at) = t.plan.as_ref().and_then(|p| p.retire_at) else {
        return false;
    };
    if clock < at {
        return false;
    }
    let near = cam.is_some_and(|c| {
        t.train
            .pose(net)
            .is_some_and(|(p, _)| c.distance(omsi_to_bevy_pos(p)) < RETIRE_FAR_M)
    });
    !near
}

/// The run of a scheduled train that is being removed, for the bookkeeping.
pub(in super::super::super) fn run_of(t: &AiTrain) -> Option<&RunRef> {
    t.plan.as_ref().map(|p| &p.run)
}
