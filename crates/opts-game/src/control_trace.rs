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
mod ray;

mod params;
use params::*;
mod set;
use set::set;
mod lookup;
use lookup::{find, region, tile};
mod export;
use export::export;
