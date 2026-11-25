//! `zones.query`: the spatial zones of the loaded tiles and the membership of the tracked subject
//! (M-0115; the payload is built by `spatial_zones::query`).

use bevy::ecs::world::World;
use bevy::prelude::With;
use opts_cli::protocol::{code, RpcError};
use serde_json::Value;

use crate::control::params::opt_u64;
use crate::control::state::{ControlRegistry, Outcome};
use crate::spatial_zones::query::zones_payload;
use crate::spatial_zones::{SpatialZone, ZoneMembership, ZoneSubject};

/// Zones listed when `limit` is absent; the count always covers all of them.
const DEFAULT_LIMIT: usize = 100;

/// Registers `zones.query`.
pub fn register(r: &mut ControlRegistry) {
    r.register("zones.query", zones_query);
}

/// The listing limit: `limit` when given (a whole number), else [`DEFAULT_LIMIT`].
fn limit_of(params: &Value) -> Result<usize, RpcError> {
    match opt_u64(params, "limit")? {
        Some(n) => usize::try_from(n).map_err(|_| {
            RpcError::new(code::INVALID_PARAMS, "limit is too large for this platform")
        }),
        None => Ok(DEFAULT_LIMIT),
    }
}

fn zones_query(world: &mut World, params: &Value) -> Outcome {
    let limit = match limit_of(params) {
        Ok(limit) => limit,
        Err(e) => return Outcome::err(e),
    };
    let zones: Vec<SpatialZone> = world.query::<&SpatialZone>().iter(world).copied().collect();
    let membership = world
        .query_filtered::<&ZoneMembership, With<ZoneSubject>>()
        .iter(world)
        .next()
        .copied();
    let subject = membership
        .as_ref()
        .and_then(|m| m.last.map(|point| (point, m)));
    tracing::debug!(
        zones = zones.len(),
        subject = subject.is_some(),
        limit,
        "zones.query answered"
    );
    Outcome::ok(zones_payload(&zones, subject, limit))
}

#[cfg(test)]
#[path = "zones_tests.rs"]
mod tests;
