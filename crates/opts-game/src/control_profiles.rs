//! Control protocol of the map profiles (M-0130): `profiles.list` and `profiles.advice` say what
//! the plan's map has, `profiles.apply`, `profiles.save` and `profiles.delete` do what the
//! *Map configuration* modal buttons do, `profiles.status` says what the running drive uses.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_str, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::map_profiles::advice::{Advice, AdviceId, Severity};
use crate::map_profiles::service::{self, Choice, ProfileView, ServiceError};
use crate::map_profiles::session::ActiveMapProfile;
use crate::map_profiles::store::ProfileSource;
use crate::map_profiles::MapProfile;

/// Registers the `profiles.*` methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("profiles.list", list);
    registry.register("profiles.advice", advice);
    registry.register("profiles.apply", apply);
    registry.register("profiles.save", save);
    registry.register("profiles.delete", delete);
    registry.register("profiles.status", status);
}

fn rpc(e: ServiceError) -> RpcError {
    let c = match e {
        ServiceError::NoMap | ServiceError::NoStore => code::NOT_READY,
        ServiceError::NotFound(_) => code::INVALID_PARAMS,
        ServiceError::Io(_) => code::INTERNAL_ERROR,
    };
    RpcError::new(c, e.to_string())
}

fn source_name(s: ProfileSource) -> &'static str {
    match s {
        ProfileSource::Chrono => "chrono",
        ProfileSource::Map => "map",
        ProfileSource::Global => "global",
    }
}

fn advice_id(id: AdviceId) -> &'static str {
    match id {
        AdviceId::GeometryAboveTypical => "geometry-above-typical",
        AdviceId::GeometryHigh => "geometry-high",
        AdviceId::LightMap => "light-map",
        AdviceId::ManySplines => "many-splines",
        AdviceId::HighAiLoad => "high-ai-load",
        AdviceId::RanSlowly => "ran-slowly",
        AdviceId::RanSmoothly => "ran-smoothly",
        AdviceId::NotPlayedYet => "not-played-yet",
    }
}

fn advice_json(a: &Advice) -> Value {
    json!({
        "id": advice_id(a.id),
        "severity": match a.severity { Severity::Info => "info", Severity::Warning => "warning" },
        "preset": a.preset,
        "suggested": a.suggested.iter().map(|(k, v)| json!({ "key": k, "value": v })).collect::<Vec<_>>(),
    })
}

fn profile_json(chrono: Option<&str>, p: &MapProfile) -> Value {
    json!({
        "chrono": chrono,
        "name": p.name,
        "created": p.created,
        "based_on": p.based_on,
        "settings": p.settings,
    })
}

fn describe(v: &ProfileView) -> Value {
    json!({
        "map": v.key.map_id,
        "chrono": v.key.chrono_id,
        "chrono_entries": v.chrono_entries,
        "resolved": {
            "source": source_name(v.resolved.source),
            "profile": v.resolved.profile.as_ref().map(|p| p.name.clone()),
            "dropped": v.resolved.dropped.iter().map(|d| json!({ "key": d.key, "reason": d.reason })).collect::<Vec<_>>(),
        },
        "picked": v.picked,
        "saved": v.saved.iter().map(|(c, p)| profile_json(Some(c), p)).collect::<Vec<_>>(),
        "presets": v.presets.iter().map(|p| profile_json(None, p)).collect::<Vec<_>>(),
        "load": {
            "tiles": v.load.tile_count,
            "objects": v.load.object_count,
            "splines": v.load.spline_count,
            "ai_pool_vehicles": v.load.ai_pool_vehicles,
            "area_km2": v.load.map_size_km2,
            "estimated": v.load.estimated,
            "counting": v.counting,
            "avg_frame_time_ms": v.load.avg_frame_time_ms,
            "p95_frame_time_ms": v.load.p95_frame_time_ms,
        },
        "advice": v.advice.iter().map(advice_json).collect::<Vec<_>>(),
    })
}

fn current_view(world: &mut World) -> Result<ProfileView, RpcError> {
    service::view(world).ok_or_else(|| rpc(ServiceError::NoMap))
}

fn list(world: &mut World, params: &Value) -> Outcome {
    list_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

fn list_impl(world: &mut World, _params: &Value) -> Result<Value, RpcError> {
    Ok(describe(&current_view(world)?))
}

fn advice(world: &mut World, params: &Value) -> Outcome {
    advice_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

fn advice_impl(world: &mut World, _params: &Value) -> Result<Value, RpcError> {
    let v = current_view(world)?;
    Ok(json!({ "load": describe(&v)["load"], "advice": describe(&v)["advice"] }))
}

/// `profiles.apply {profile, make_default?}`: `profile` is `none`, `preset:<id>` or the name of a
/// saved profile of the map; it is used for the next drive of the plan's map.
fn apply(world: &mut World, params: &Value) -> Outcome {
    apply_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

fn apply_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let which = str_param(params, "profile")?;
    let choice = if which.eq_ignore_ascii_case("none") || which.eq_ignore_ascii_case("global") {
        Choice::Global
    } else if let Some(id) = which.strip_prefix("preset:") {
        Choice::Preset(id.to_string())
    } else {
        Choice::Saved(which.to_string())
    };
    let make_default = opt_bool(params, "make_default")?.unwrap_or(false);
    service::apply(world, &choice, make_default).map_err(rpc)?;
    Ok(describe(&current_view(world)?))
}

/// `profiles.save {name, chrono?}`: a profile of the current global values for the map's default,
/// or for the active Chrono entry with `chrono: true`.
fn save(world: &mut World, params: &Value) -> Outcome {
    save_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

fn save_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let name = str_param(params, "name")?.trim().to_string();
    if name.is_empty() {
        return Err(RpcError::new(
            code::INVALID_PARAMS,
            "the profile needs a name",
        ));
    }
    let chrono = opt_bool(params, "chrono")?.unwrap_or(false);
    service::create_from_current(world, &name, chrono).map_err(rpc)?;
    Ok(describe(&current_view(world)?))
}

/// `profiles.delete {chrono?}`: removes the profile file of the key.
fn delete(world: &mut World, params: &Value) -> Outcome {
    delete_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

fn delete_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let chrono = opt_bool(params, "chrono")?.unwrap_or(false);
    let removed = service::delete(world, chrono).map_err(rpc)?;
    let mut v = describe(&current_view(world)?);
    v["removed"] = json!(removed);
    Ok(v)
}

/// What the running drive uses.
fn status(world: &mut World, params: &Value) -> Outcome {
    status_impl(world, params).map_or_else(Outcome::err, Outcome::ok)
}

fn status_impl(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let _ = opt_str(params, "unused")?;
    let a = world.resource::<ActiveMapProfile>();
    Ok(json!({
        "active": a.name.is_some(),
        "map": a.key.as_ref().map(|k| k.map_id.clone()),
        "chrono": a.key.as_ref().map(|k| k.chrono_id.clone()),
        "profile": a.name,
        "source": a.source.map(source_name),
        "keys": a.keys,
        "overlaid": world.resource::<crate::settings::GameSettings>().overlaid_keys(),
    }))
}
