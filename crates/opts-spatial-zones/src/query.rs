//! The JSON payload of the `zones.query` control method, the facade over
//! `opts-spatial-zones-core::query` (ISS-092). The handler that serves it is
//! `control::handlers::zones` in `opts-bevy`.

use bevy::math::Vec2;
use serde_json::Value;

use super::model::SpatialZone;
use super::sampler::ZoneMembership;
use opts_spatial_zones_core::query::SubjectSample;

pub use opts_spatial_zones_core::query::kind_label;

/// `{zones, count, subject}`: the core's payload, with the subject read from its membership.
#[must_use]
pub fn zones_payload(
    zones: &[SpatialZone],
    subject: Option<(Vec2, &ZoneMembership)>,
    limit: usize,
) -> Value {
    let subject = subject.map(|(point, m)| SubjectSample {
        point,
        inside: m.tracker.inside(),
        zone: m.zone,
    });
    opts_spatial_zones_core::query::zones_payload(zones, subject, limit)
}

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
