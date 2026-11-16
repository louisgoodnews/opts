//! The 10 Hz membership sampler: for each subject, tests its ground point against the zones and
//! feeds the result to its [`ZoneTracker`], emitting [`ZoneEntered`] / [`ZoneLeft`] on transitions.

use bevy::math::Vec2;
use bevy::prelude::*;

use super::model::{SpatialZone, ZoneEntered, ZoneId, ZoneKind, ZoneLeft};
use super::tracker::{SampleGate, Transition, ZoneTracker};

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

/// What one sample produced for a subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emitted {
    Entered(ZoneId, ZoneKind),
    Left(ZoneId, ZoneKind),
}

/// Advances `membership` by one sample at ground point `p` against `zones`. Returns the
/// transition, if this sample completes one. A subject that moves from one zone straight into
/// another stays inside without a transition: zones that touch are one covered stretch.
pub fn step(
    membership: &mut ZoneMembership,
    p: Vec2,
    zones: &[SpatialZone],
) -> Option<Emitted> {
    let hit = zones.iter().find(|z| z.contains(p));
    let advance = membership.last.map_or(0.0, |last| last.distance(p));
    membership.last = Some(p);
    let transition = membership.tracker.update(hit.is_some(), advance)?;
    match transition {
        Transition::Entered => {
            let zone = hit?;
            membership.zone = Some((zone.id, zone.kind));
            Some(Emitted::Entered(zone.id, zone.kind))
        }
        Transition::Left => {
            let (id, kind) = membership.zone.take()?;
            Some(Emitted::Left(id, kind))
        }
    }
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
