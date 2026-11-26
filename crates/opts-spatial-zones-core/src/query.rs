//! The JSON payload of the `zones.query` control method: the zones of the loaded tiles and the
//! membership of the subject. Pure data in, JSON out; the handler that serves it is
//! `control::handlers::zones` in `opts-bevy`.

use glam::Vec2;
use serde_json::{json, Value};

use crate::model::{ZoneId, ZoneKind, ZoneShape};

/// The membership of the tracked subject, as the payload reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SubjectSample {
    /// The last sampled ground point.
    pub point: Vec2,
    /// Whether the subject is inside.
    pub inside: bool,
    /// The zone the subject is in, when it is inside one.
    pub zone: Option<(ZoneId, ZoneKind)>,
}

/// The label of a zone kind in the protocol.
#[must_use]
pub fn kind_label(kind: ZoneKind) -> &'static str {
    match kind {
        ZoneKind::Tunnel => "tunnel",
    }
}

fn zone_json<Z: ZoneShape>(zone: &Z) -> Value {
    json!({
        "id": zone.zone_id().0,
        "kind": kind_label(zone.zone_kind()),
        "min": [zone.zone_min().x, zone.zone_min().y],
        "max": [zone.zone_max().x, zone.zone_max().y],
    })
}

/// `{zones, count, subject}`. `subject` is `null` when no subject is tracked; otherwise it holds
/// the sampled point, whether the subject is inside and the zone it is in.
#[must_use]
pub fn zones_payload<Z: ZoneShape>(
    zones: &[Z],
    subject: Option<SubjectSample>,
    limit: usize,
) -> Value {
    let subject = subject.map(|s| {
        json!({
            "point": [s.point.x, s.point.y],
            "inside": s.inside,
            "zone": s.zone.map(|(id, kind)| json!({"id": id.0, "kind": kind_label(kind)})),
        })
    });
    json!({
        "count": zones.len(),
        "zones": zones.iter().take(limit).map(zone_json).collect::<Vec<_>>(),
        "subject": subject,
    })
}
