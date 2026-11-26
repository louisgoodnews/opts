//! Lane changes and strategic changes, split by concern.

use super::*;

mod announce;
mod overtake;
mod rules;

/// Two parallel north-bound lanes 3.5 m apart: lane 0 (left) and lane 1 (right, east).
pub(super) fn two_lanes() -> TrafficWorld {
    let right = LaneSegment::straight(DVec2::new(3.5, 0.0), 0.0, 800.0, LaneKind::Road);
    world(&[straight(0.0, 800.0), right])
}

pub(super) fn slow() -> IdmParams {
    IdmParams {
        desired_speed: 3.0,
        ..IdmParams::default()
    }
}
