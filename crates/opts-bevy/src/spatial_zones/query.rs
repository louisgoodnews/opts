//! The JSON payload of the `zones.query` control method: the zones of the loaded tiles and the
//! membership of the subject. The handler that serves it is `control::handlers::zones`.

use bevy::math::Vec2;
use serde_json::{json, Value};

use super::model::{SpatialZone, ZoneKind};
use super::sampler::ZoneMembership;

/// The label of a zone kind in the protocol.
#[must_use]
pub fn kind_label(kind: ZoneKind) -> &'static str {
    match kind {
        ZoneKind::Tunnel => "tunnel",
    }
}

fn zone_json(zone: &SpatialZone) -> Value {
    json!({
        "id": zone.id.0,
        "kind": kind_label(zone.kind),
        "min": [zone.min.x, zone.min.y],
        "max": [zone.max.x, zone.max.y],
    })
}

/// `{zones, count, subject}`. `subject` is `null` when no subject is tracked; otherwise it holds
/// the sampled point, whether the subject is inside and the zone it is in.
#[must_use]
pub fn zones_payload(
    zones: &[SpatialZone],
    subject: Option<(Vec2, &ZoneMembership)>,
    limit: usize,
) -> Value {
    let subject = subject.map(|(p, m)| {
        json!({
            "point": [p.x, p.y],
            "inside": m.tracker.inside(),
            "zone": m.zone.map(|(id, kind)| json!({"id": id.0, "kind": kind_label(kind)})),
        })
    });
    json!({
        "count": zones.len(),
        "zones": zones.iter().take(limit).map(zone_json).collect::<Vec<_>>(),
        "subject": subject,
    })
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
