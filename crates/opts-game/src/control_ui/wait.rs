//! `ui.wait_for`: the condition vocabulary (`Cond`, validation) and the waiting method.

use std::time::{Duration, Instant};

use opts_bevy::control::params::{opt_f64, opt_str, opt_u64, str_param};
use opts_bevy::control::state::Outcome;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};
use bevy::prelude::*;

use super::eval::eval;

#[derive(Debug, Clone)]
pub(super) struct Cond {
    pub(super) until: String,
    pub(super) id: Option<String>,
    pub(super) value: Option<String>,
    pub(super) number: Option<f64>,
}

fn need<'a>(c: &'a Cond, what: &str) -> Result<&'a str, RpcError> {
    match what {
        "id" => c.id.as_deref(),
        _ => c.value.as_deref(),
    }
    .ok_or_else(|| RpcError::invalid_params(format!("`until: {}` needs `{what}`", c.until)))
}

fn num(c: &Cond) -> Result<f64, RpcError> {
    c.number
        .ok_or_else(|| RpcError::invalid_params(format!("`until: {}` needs `number`", c.until)))
}

pub(super) const KNOWN: &[&str] = &[
    "exists",
    "visible",
    "gone",
    "enabled",
    "disabled",
    "focused",
    "checked",
    "text",
    "screen",
    "dialog",
    "no_dialog",
    "app_state",
    "hud_ready",
    "speed_above",
    "speed_below",
    "distance_above",
    "paused",
    "running",
    "sim_time_above",
    "items_above",
];

pub(super) fn validate(c: &Cond) -> Result<(), RpcError> {
    if !KNOWN.contains(&c.until.as_str()) {
        return Err(
            RpcError::invalid_params(format!("unknown condition `{}`", c.until))
                .with_data(json!({ "valid": KNOWN })),
        );
    }
    match c.until.as_str() {
        "exists" | "visible" | "gone" | "enabled" | "disabled" | "focused" | "checked" => {
            need(c, "id")?;
        }
        "items_above" => {
            need(c, "id")?;
            num(c)?;
        }
        "text" => {
            need(c, "id")?;
            need(c, "value")?;
        }
        "screen" | "app_state" => {
            need(c, "value")?;
        }
        "speed_above" | "speed_below" | "distance_above" | "sim_time_above" => {
            num(c)?;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn wait_for(_world: &mut World, params: &Value) -> Outcome {
    let parsed = (|| -> Result<(Cond, u64), RpcError> {
        let cond = Cond {
            until: str_param(params, "until")?.to_string(),
            id: opt_str(params, "id")?.map(str::to_string),
            value: opt_str(params, "value")?.map(str::to_string),
            number: opt_f64(params, "number")?,
        };
        validate(&cond)?;
        let ms = opt_u64(params, "timeout_ms")?.unwrap_or(10_000);
        if ms > 120_000 {
            return Err(RpcError::invalid_params("`timeout_ms` is at most 120000"));
        }
        Ok((cond, ms))
    })();
    let (cond, timeout_ms) = match parsed {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let began = Instant::now();
    let deadline = Duration::from_millis(timeout_ms);
    let mut last_eval = Instant::now() - Duration::from_secs(1);
    let mut last = json!(null);
    Outcome::Wait(Box::new(move |world| {
        let (holds, observed) = if last_eval.elapsed() >= Duration::from_millis(40) {
            last_eval = Instant::now();
            eval(world, &cond)
        } else {
            (false, last.clone())
        };
        last = observed;
        if holds {
            return Some(Ok(json!({
                "ok": true,
                "until": cond.until,
                "waited_ms": began.elapsed().as_millis() as u64,
                "observed": last,
            })));
        }
        (began.elapsed() >= deadline).then(|| {
            Err(RpcError::new(
                code::TIMEOUT,
                format!("`{}` did not hold within {timeout_ms} ms", cond.until),
            )
            .with_data(
                json!({ "last": last, "until": cond.until, "id": cond.id, "value": cond.value }),
            ))
        })
    }))
}
