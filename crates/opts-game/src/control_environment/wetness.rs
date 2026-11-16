//! The wetness part of `sim.set_time` (M-0121, the hold since 1.35): `wetness` sets the global
//! wetness, `hold: true` keeps it through the drive (`WetnessHold`), `release: true` lets the
//! weather's ramp take it again.

use bevy::prelude::*;
use opts_bevy::control::params::{opt_bool, opt_f64};
use opts_bevy::reflections::{Wetness, WetnessHold};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

/// The wetness part of one `sim.set_time` request.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct WetnessRequest {
    /// `wetness`, clamped to `0..=1`; required with `hold: true`.
    pub wetness: Option<f32>,
    pub hold: bool,
    pub release: bool,
}

/// The `wetness` parameter (M-0121), clamped to `0..=1`; `None` when absent.
pub fn wetness_param(params: &Value) -> Result<Option<f32>, RpcError> {
    Ok(opt_f64(params, "wetness")?.map(|w| w.clamp(0.0, 1.0) as f32))
}

/// The request's wetness parameters, checked: `hold` and `release` exclude each other, and a
/// hold needs the value to hold.
///
/// # Errors
/// `-32602` for a hold without `wetness`, or with `release`, and for a malformed parameter.
pub fn request(params: &Value) -> Result<WetnessRequest, RpcError> {
    let wetness = wetness_param(params)?;
    let hold = opt_bool(params, "hold")?.unwrap_or(false);
    let release = opt_bool(params, "release")?.unwrap_or(false);
    if hold && release {
        return Err(RpcError::invalid_params(
            "`hold` and `release` cannot both be true",
        ));
    }
    if hold && wetness.is_none() {
        return Err(RpcError::invalid_params(
            "`hold: true` needs `wetness` (the value to hold)",
        ));
    }
    Ok(WetnessRequest {
        wetness,
        hold,
        release,
    })
}

/// The hold after the request: `None` leaves it as it is; `Some(next)` sets it. `hold` sets it to
/// the wetness, `release` clears it, and a plain `wetness` (set once) clears it too, so the
/// weather's ramp resumes.
#[must_use]
pub fn hold_change(r: &WetnessRequest) -> Option<Option<f32>> {
    if r.hold {
        Some(r.wetness)
    } else if r.release || r.wetness.is_some() {
        Some(None)
    } else {
        None
    }
}

/// Applies the request to the world: the hold, then the global wetness. Nothing changes when the
/// global wetness is missing and a wetness was asked for.
///
/// # Errors
/// `NOT_READY` without a `Wetness` resource (no drive) when `wetness` is given.
pub fn apply(world: &mut World, r: &WetnessRequest) -> Result<(), RpcError> {
    if r.wetness.is_some() && !world.contains_resource::<Wetness>() {
        return Err(RpcError::new(
            code::NOT_READY,
            "no wetness state (start a drive first)",
        ));
    }
    if let Some(next) = hold_change(r) {
        world.insert_resource(WetnessHold(next));
    }
    if let (Some(w), Some(mut global)) = (r.wetness, world.get_resource_mut::<Wetness>()) {
        global.0 = w;
    }
    Ok(())
}

/// The hold for the answer: its value, or `null` when nothing is held.
#[must_use]
pub fn hold_of(world: &World) -> Value {
    json!(world.get_resource::<WetnessHold>().and_then(|h| h.0))
}
