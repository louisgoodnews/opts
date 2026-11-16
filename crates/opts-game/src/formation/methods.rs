//! The control-protocol methods `formation.validate` and `formation.spawn`.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_str, opt_u64};
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use opts_core::formats::trains::FormationIssue;
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldLoadJob;
use crate::session_vehicles::{EntrySel, SessionVehicles};

use super::lookup::JobLookup;
use super::request::{formation_from, session_up};
use super::resolve::{queue_spawn, resolve};

pub(super) fn issues_json(issues: &[FormationIssue]) -> Vec<Value> {
    issues
        .iter()
        .map(|i| json!({ "key": i.key(), "car": i.car(), "message": i.to_string() }))
        .collect()
}

pub(super) fn validate(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        let formation = formation_from(world, params)?;
        let job = world
            .get_resource::<WorldLoadJob>()
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
        let issues = formation.validate(&JobLookup(job));
        Ok(json!({
            "ok": issues.is_empty(),
            "name": formation.name,
            "cars": formation.len(),
            "issues": issues_json(&issues),
        }))
    })();
    result.into()
}

pub(super) fn spawn(world: &mut World, params: &Value) -> Outcome {
    let result = (|| -> Result<Value, RpcError> {
        session_up(world)?;
        let formation = formation_from(world, params)?;
        let resolved = {
            let job = world
                .get_resource::<WorldLoadJob>()
                .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
            resolve(&formation, job)
        };
        let resolved = resolved.map_err(|issues| {
            RpcError::new(
                code::INVALID_PARAMS,
                issues
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        let entry = match (opt_u64(params, "entry_id")?, opt_str(params, "entry")?) {
            (Some(i), _) => EntrySel::Id(
                u32::try_from(i).map_err(|_| RpcError::invalid_params("`entry_id` too large"))?,
            ),
            (None, Some(n)) => EntrySel::Name(n.to_owned()),
            (None, None) => EntrySel::Default,
        };
        let id = queue_spawn(
            &mut world.resource_mut::<SessionVehicles>(),
            &formation,
            &resolved,
            entry,
        );
        Ok(json!({ "id": id, "queued": true, "cars": resolved.cars.len() + 1 }))
    })();
    result.into()
}
