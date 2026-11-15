//! Change-map planning and the entry and stop parameters of a session.


use super::*;


/// `session.change_map {map, entry?, vehicle?, dry_run?}`: the Alt menu's *Change map*.
pub(super) fn change_map(world: &mut World, params: &Value) -> Outcome {
    use crate::in_game::change_map::{plan_in_world, start, ChangeMapError, VehicleChoice};
    let result = (|| -> Result<Value, RpcError> {
        let map = opts_bevy::control::params::str_param(params, "map")?;
        let entry = opt_str(params, "entry")?;
        let choice = match opt_str(params, "vehicle")? {
            None | Some("carry") => VehicleChoice::Carry,
            Some("none") => VehicleChoice::None,
            Some(_) => {
                return Err(RpcError::invalid_params("`vehicle` is `carry` or `none`"));
            }
        };
        let dry_run = opt_bool(params, "dry_run")?.unwrap_or(false);
        let plan = plan_in_world(world, map, entry, choice).map_err(|e| match e {
            ChangeMapError::NotInDrive | ChangeMapError::NoCatalogue => {
                RpcError::new(code::NOT_READY, e.to_string())
            }
            ChangeMapError::VehicleUnavailable(_) => RpcError::new(code::NOT_FOUND, e.to_string()),
            _ => RpcError::invalid_params(e.to_string()),
        })?;
        let mut answer = plan.describe();
        answer["started"] = json!(!dry_run);
        if !dry_run {
            start(world, &plan);
        }
        Ok(answer)
    })();
    Outcome::Done(result)
}

/// The entry point a spawn asks for: `entry_id`, else `entry` (a name), else the session's own.
pub(crate) fn entry_param(params: &Value) -> Result<EntrySel, RpcError> {
    Ok(
        match (opt_u64(params, "entry_id")?, opt_str(params, "entry")?) {
            (Some(i), _) => EntrySel::Id(
                u32::try_from(i).map_err(|_| RpcError::invalid_params("`entry_id` too large"))?,
            ),
            (None, Some(n)) => EntrySel::Name(n.to_owned()),
            (None, None) => EntrySel::Default,
        },
    )
}

/// `stop`: a bus stop's station id, or its name in the map's `Busstops.cfg` (exact, ignoring
/// case, else the first containing it).
pub(super) fn stop_param(world: &mut World, params: &Value) -> Result<Option<u32>, RpcError> {
    match params.get("stop") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| RpcError::invalid_params("`stop` must be a station id or a name")),
        Some(Value::String(name)) => {
            if let Ok(n) = name.trim().parse::<u32>() {
                return Ok(Some(n));
            }
            let tt = world
                .get_resource_mut::<crate::in_game::omsi_world::WorldLoadJob>()
                .and_then(|mut j| j.timetables())
                .ok_or_else(|| {
                    RpcError::new(code::NOT_FOUND, "the map has no timetable stop list")
                })?;
            let want = name.trim().to_lowercase();
            tt.stops
                .iter()
                .find(|s| s.name.to_lowercase() == want)
                .or_else(|| {
                    tt.stops
                        .iter()
                        .find(|s| s.name.to_lowercase().contains(&want))
                })
                .map(|s| Some(s.id))
                .ok_or_else(|| RpcError::new(code::NOT_FOUND, format!("no bus stop `{name}`")))
        }
        Some(_) => Err(RpcError::invalid_params(
            "`stop` must be a station id or a name",
        )),
    }
}

pub(crate) fn session_ready(world: &World) -> Result<(), RpcError> {
    session_up(world)
}
