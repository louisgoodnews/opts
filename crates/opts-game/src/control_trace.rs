//! Control protocol of the scene trace (M-0087): `trace.set`, `trace.pick`, `trace.tile`,
//! `trace.find`, `trace.region`, `trace.export`, plus the in-world overlay of the developer panel.
//!
//! See `docs/src/architecture/scene-trace.md`.

use std::path::PathBuf;

use bevy::prelude::*;
use opts_bevy::control::params::{f64_param, opt_bool, opt_f64, opt_str, opt_u64, str_param};
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_bevy::control::PathPolicy;
use opts_bevy::scene_trace::{self as st, Filters, Mode, Query as TraceQuery, Radius};
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldStats;

/// Registers the `trace.*` methods.
pub fn register(registry: &mut ControlRegistry) {
    registry.register("trace.set", set);
    registry.register("trace.pick", pick);
    registry.register("trace.tile", tile);
    registry.register("trace.find", find);
    registry.register("trace.region", region);
    registry.register("trace.export", export);
}

mod overlay;
pub use overlay::{add_overlay, TraceOverlay};

mod filters;
mod read;
use filters::*;
use read::*;


fn not_on() -> RpcError {
    RpcError::new(
        code::NOT_READY,
        "the scene trace is off; switch it on first (`trace.set mode=full`, or the setting, or --trace-scene) and load the map again",
    )
}
fn status_json() -> Value {
    let mut v = serde_json::to_value(st::status()).unwrap_or(Value::Null);
    if let Value::Object(m) = &mut v {
        m.insert("filters".into(), filters_json(&st::filters()));
        if m.get("mode").and_then(Value::as_str) == Some("") {
            m.insert("mode".into(), json!("off"));
        }
    }
    v
}

/// `trace.set {mode?, tiles?, kinds?, file?, radius?, center?, size_cap_mb?, overlay?, reset?}`.
fn set(world: &mut World, params: &Value) -> Outcome {
    let mode = match opt_str(params, "mode") {
        Ok(Some(m)) => match Mode::parse(m) {
            Some(m) => Some(m),
            None => {
                return Outcome::err(RpcError::invalid_params(
                    "`mode` must be off, summary or full",
                ))
            }
        },
        Ok(None) => None,
        Err(e) => return Outcome::err(e),
    };
    let filters = match parse_filters(world, params, st::filters()) {
        Ok(f) => f,
        Err(e) => return Outcome::err(e),
    };
    if let Some(mode) = mode {
        st::set_mode(mode);
    }
    if st::enabled() {
        st::set_filters(filters);
        if let Ok(Some(mb)) = opt_u64(params, "size_cap_mb") {
            st::set_size_cap(mb.max(1) * 1024 * 1024);
        }
    }
    if let Ok(Some(on)) = opt_bool(params, "overlay") {
        if let Some(mut o) = world.get_resource_mut::<TraceOverlay>() {
            o.enabled = on;
        }
    }
    let mut out = status_json();
    out["overlay"] = json!(world
        .get_resource::<TraceOverlay>()
        .is_some_and(|o| o.enabled));
    Outcome::ok(out)
}

fn record_json(r: &st::TraceRecord) -> Value {
    serde_json::to_value(r).unwrap_or(Value::Null)
}

fn summary_row(r: &st::TraceRecord) -> Value {
    json!({
        "id": r.id, "kind": r.kind, "tile": r.tile, "file": r.file,
        "result": r.result, "quality": r.quality(),
        "meshes": r.meshes.iter().map(|m| json!({"file": m.file, "stage": m.stage})).collect::<Vec<_>>(),
        "warnings": r.warnings, "position": r.world.map(|w| w.t),
    })
}

mod query;
use query::{pick, rows, run_query};
