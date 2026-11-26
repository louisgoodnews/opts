//! The 10 Hz membership sampler: for each subject, tests its ground point against the zones and
//! feeds the result to its [`ZoneTracker`], emitting [`ZoneEntered`] / [`ZoneLeft`] on transitions.
//! The step itself is pure and lives in `opts-spatial-zones-core` (ISS-092); this module is the
//! Bevy wrapper that holds the components and writes the messages.

use bevy::math::Vec2;
use bevy::prelude::*;

use super::model::{SpatialZone, ZoneEntered, ZoneLeft};
use opts_spatial_zones_core::model::{ZoneId, ZoneKind};
use opts_spatial_zones_core::sampler as core_sampler;
use opts_spatial_zones_core::tracker::{SampleGate, ZoneTracker};

pub use opts_spatial_zones_core::sampler::Emitted;

/// Marks an entity whose zone membership is tracked (the player vehicle, the camera).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ZoneSubject;

/// The per-subject tracking state: the hysteresis tracker, the last sampled point and the zone
/// the subject is in.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ZoneMembership {
    pub tracker: ZoneTracker,
    pub last: Option<Vec2>,
    pub zone: Option<(ZoneId, ZoneKind)>,
}

impl Default for ZoneMembership {
    fn default() -> Self {
        Self::new(15.0, 5.0)
    }
}

impl ZoneMembership {
    /// A membership outside all zones with the given enter and leave distances (metres).
    #[must_use]
    pub fn new(enter_m: f32, leave_m: f32) -> Self {
        Self {
            tracker: ZoneTracker::new(enter_m, leave_m),
            last: None,
            zone: None,
        }
    }
}

/// Advances `membership` by one sample at ground point `p` against `zones`. Delegates to the
/// core's step, on the membership's fields.
pub fn step(membership: &mut ZoneMembership, p: Vec2, zones: &[SpatialZone]) -> Option<Emitted> {
    core_sampler::step(
        &mut membership.tracker,
        &mut membership.last,
        &mut membership.zone,
        p,
        zones,
    )
}

/// Samples every subject at most [`SampleGate::RATE_HZ`] times per second.
pub fn sample_zone_subjects(
    time: Res<Time>,
    mut gate: Local<SampleGate>,
    zones: Query<&SpatialZone>,
    mut subjects: Query<(&Transform, &mut ZoneMembership), With<ZoneSubject>>,
    mut entered: MessageWriter<ZoneEntered>,
    mut left: MessageWriter<ZoneLeft>,
) {
    if !gate.tick(time.delta_secs()) {
        return;
    }
    let zones: Vec<SpatialZone> = zones.iter().copied().collect();
    for (transform, mut membership) in &mut subjects {
        let p = Vec2::new(transform.translation.x, transform.translation.z);
        match step(&mut membership, p, &zones) {
            Some(Emitted::Entered(zone, kind)) => {
                entered.write(ZoneEntered { zone, kind });
            }
            Some(Emitted::Left(zone, kind)) => {
                left.write(ZoneLeft { zone, kind });
            }
            None => {}
        }
    }
}

#[cfg(test)]
#[path = "sampler_tests.rs"]
mod tests;
