//! The pure part of the 10 Hz membership sampler: one step of a subject's membership against the
//! zones, and the [`Emitted`] transition it reports. The Bevy system that reads the world and
//! writes the messages stays in `opts-spatial-zones` (ISS-092).

use glam::Vec2;

use crate::model::{ZoneId, ZoneKind, ZoneShape};
use crate::tracker::{Transition, ZoneTracker};

/// What one sample produced for a subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Emitted {
    Entered(ZoneId, ZoneKind),
    Left(ZoneId, ZoneKind),
}

/// Advances one subject's membership by one sample at ground point `p` against `zones`. The
/// subject's state is the tracker, the last sampled point and the zone it is in. Returns the
/// transition, if this sample completes one. A subject that moves from one zone straight into
/// another stays inside without a transition: zones that touch are one covered stretch.
pub fn step<Z: ZoneShape>(
    tracker: &mut ZoneTracker,
    last: &mut Option<Vec2>,
    zone: &mut Option<(ZoneId, ZoneKind)>,
    p: Vec2,
    zones: &[Z],
) -> Option<Emitted> {
    let hit = zones.iter().find(|z| z.contains_point(p));
    let advance = (*last).map_or(0.0, |prev| prev.distance(p));
    *last = Some(p);
    let transition = tracker.update(hit.is_some(), advance)?;
    match transition {
        Transition::Entered => {
            let entered = hit?;
            *zone = Some((entered.zone_id(), entered.zone_kind()));
            Some(Emitted::Entered(entered.zone_id(), entered.zone_kind()))
        }
        Transition::Left => {
            let (id, kind) = zone.take()?;
            Some(Emitted::Left(id, kind))
        }
    }
}
