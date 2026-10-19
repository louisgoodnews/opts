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

/// The overlay of the developer panel: wire boxes coloured by decode quality around the camera.
#[derive(Resource, Debug, Clone, Default)]
pub struct TraceOverlay {
    /// Whether the overlay draws.
    pub enabled: bool,
    /// The record highlighted (the last `trace.pick`).
    pub picked: Option<u64>,
    /// Radius around the camera (m); 0 uses the default.
    pub radius: f32,
}

/// Default radius of the overlay (m).
const OVERLAY_RADIUS_M: f32 = 120.0;
/// Most boxes the overlay draws in one frame.
const OVERLAY_MAX_BOXES: usize = 1500;

/// Adds the overlay resource and system.
pub fn add_overlay(app: &mut App) {
    app.init_resource::<TraceOverlay>().add_systems(
        PostUpdate,
        draw_overlay.run_if(resource_exists::<bevy::gizmos::config::GizmoConfigStore>),
    );
}

fn colour(quality: &str, kind: &str) -> Color {
    match quality {
        "failed" => Color::srgb(1.0, 0.1, 0.1),
        "unreliable" => Color::srgb(1.0, 0.6, 0.0),
        "decoded" => Color::srgb(1.0, 0.9, 0.2),
        _ => match kind {
            st::kind::PARKED | st::kind::TRAFFIC | st::kind::VEHICLE => Color::srgb(0.3, 0.7, 1.0),
            st::kind::HUMAN => Color::srgb(0.8, 0.4, 1.0),
            st::kind::TREE => Color::srgb(0.2, 0.8, 0.3),
            _ => Color::srgb(0.6, 0.9, 0.6),
        },
    }
}

fn draw_overlay(
    overlay: Res<TraceOverlay>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
    mut gizmos: Gizmos,
) {
    if !overlay.enabled || !st::enabled() {
        return;
    }
    let Ok(cam) = camera.single() else { return };
    let radius = if overlay.radius > 0.0 {
        overlay.radius
    } else {
        OVERLAY_RADIUS_M
    };
    let c = cam.translation();
    for r in st::overlay_snapshot([c.x, c.z], radius, OVERLAY_MAX_BOXES) {
        let (Some(w), Some(b)) = (r.world, r.bounds) else {
            continue;
        };
        let (min, max) = (Vec3::from(b.min), Vec3::from(b.max));
        let local = Transform::from_translation((min + max) * 0.5)
            .with_scale((max - min).max(Vec3::splat(0.05)));
        let world = Transform {
            translation: Vec3::from(w.t),
            rotation: Quat::from_xyzw(w.q[0], w.q[1], w.q[2], w.q[3]),
            scale: Vec3::from(w.s),
        };
        let m = world.to_matrix() * local.to_matrix();
        let picked = overlay.picked == Some(r.id);
        let col = if picked {
            Color::WHITE
        } else {
            colour(r.quality(), &r.kind)
        };
        gizmos.cube(Transform::from_matrix(m), col);
    }
}

fn not_on() -> RpcError {
    RpcError::new(
        code::NOT_READY,
        "the scene trace is off; switch it on first (`trace.set mode=full`, or the setting, or --trace-scene) and load the map again",
    )
}

fn limit_param(params: &Value, default: u64) -> usize {
    opt_u64(params, "limit")
        .ok()
        .flatten()
        .unwrap_or(default)
        .clamp(1, 5000) as usize
}

fn tile_param(params: &Value) -> Result<Option<(i32, i32)>, RpcError> {
    let Some(v) = params.get("tile").filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let bad = || RpcError::invalid_params("`tile` must be [x, y]");
    let a = v.as_array().ok_or_else(bad)?;
    if a.len() != 2 {
        return Err(bad());
    }
    let n = |i: usize| {
        a[i].as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(bad)
    };
    Ok(Some((n(0)?, n(1)?)))
}

fn camera_xz(world: &mut World) -> Option<[f32; 2]> {
    let mut q = world.query_filtered::<&GlobalTransform, With<opts_bevy::MainCamera>>();
    q.iter(world)
        .next()
        .map(|t| [t.translation().x, t.translation().z])
}

fn parse_filters(world: &mut World, params: &Value, base: Filters) -> Result<Filters, RpcError> {
    let mut f = base;
    if params.get("reset").and_then(Value::as_bool) == Some(true) {
        f = Filters::default();
    }
    if let Some(v) = params.get("tiles") {
        f.tiles = match v {
            Value::Null => Vec::new(),
            Value::Array(a) => a
                .iter()
                .map(|t| {
                    let p = t.as_array().filter(|p| p.len() == 2);
                    let n = |i: usize| {
                        p.and_then(|p| p[i].as_i64())
                            .and_then(|n| i32::try_from(n).ok())
                    };
                    n(0).zip(n(1))
                        .ok_or_else(|| RpcError::invalid_params("`tiles` must be [[x, y], ...]"))
                })
                .collect::<Result<_, _>>()?,
            _ => return Err(RpcError::invalid_params("`tiles` must be [[x, y], ...]")),
        };
    }
    if let Some(v) = params.get("kinds") {
        f.kinds = match v {
            Value::Null => Vec::new(),
            Value::Array(a) => a
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            Value::String(s) => s.split(',').map(|k| k.trim().to_owned()).collect(),
            _ => return Err(RpcError::invalid_params("`kinds` must be a list of kinds")),
        };
        if let Some(bad) = f
            .kinds
            .iter()
            .find(|k| !st::kind::ALL.contains(&k.as_str()))
        {
            return Err(RpcError::invalid_params(format!(
                "unknown kind `{bad}` (one of {})",
                st::kind::ALL.join(", ")
            )));
        }
    }
    if params.get("file").is_some() {
        f.file = opt_str(params, "file")?
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
    }
    if params.get("radius").is_some() {
        f.radius = match opt_f64(params, "radius")? {
            Some(m) if m > 0.0 => {
                let center = match params.get("center").and_then(Value::as_array) {
                    Some(c) if c.len() == 2 => [
                        c[0].as_f64().unwrap_or(0.0) as f32,
                        c[1].as_f64().unwrap_or(0.0) as f32,
                    ],
                    _ => camera_xz(world).unwrap_or([0.0, 0.0]),
                };
                Some(Radius {
                    center,
                    meters: m as f32,
                })
            }
            _ => None,
        };
    }
    Ok(f)
}

fn filters_json(f: &Filters) -> Value {
    json!({
        "tiles": f.tiles, "kinds": f.kinds, "file": f.file,
        "radius": f.radius.map(|r| json!({"center": r.center, "meters": r.meters})),
    })
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

/// `trace.tile {tile, kind?, file?, stage?, issues_only?, full?, limit?}`: counts and records.
fn tile(_: &mut World, params: &Value) -> Outcome {
    if !st::enabled() {
        return Outcome::err(not_on());
    }
    let key = match tile_param(params) {
        Ok(Some(k)) => k,
        Ok(None) => return Outcome::err(RpcError::invalid_params("missing parameter `tile`")),
        Err(e) => return Outcome::err(e),
    };
    let q = match run_query(
        params,
        TraceQuery {
            tile: Some(key),
            ..TraceQuery::default()
        },
    ) {
        Ok(q) => q,
        Err(e) => return Outcome::err(e),
    };
    let full = opt_bool(params, "full").ok().flatten().unwrap_or(false);
    let records = st::query(&q);
    Outcome::ok(json!({
        "tile": [key.0, key.1],
        "summary": st::tile_summary(key),
        "count": records.len(),
        "records": rows(&records, full),
    }))
}

/// `trace.find {file?, kind?, stage?, tile?, issues_only?, full?, limit?}`.
fn find(_: &mut World, params: &Value) -> Outcome {
    if !st::enabled() {
        return Outcome::err(not_on());
    }
    let tile = match tile_param(params) {
        Ok(t) => t,
        Err(e) => return Outcome::err(e),
    };
    let q = match run_query(
        params,
        TraceQuery {
            tile,
            ..TraceQuery::default()
        },
    ) {
        Ok(q) => q,
        Err(e) => return Outcome::err(e),
    };
    if q.file.is_none()
        && q.kind.is_none()
        && q.stage.is_none()
        && !q.issues_only
        && q.tile.is_none()
    {
        return Outcome::err(RpcError::invalid_params(
            "give at least one of `file`, `kind`, `stage`, `tile`, `issues_only`",
        ));
    }
    let full = opt_bool(params, "full").ok().flatten().unwrap_or(false);
    let records = st::query(&q);
    Outcome::ok(json!({"count": records.len(), "records": rows(&records, full)}))
}

/// `trace.region {x?, z?, radius, kind?, ...}`: records within a radius of a point on the ground
/// (Bevy world x, z as in the records' `world.t`); the camera when no point is given.
fn region(world: &mut World, params: &Value) -> Outcome {
    if !st::enabled() {
        return Outcome::err(not_on());
    }
    let radius = match f64_param(params, "radius") {
        Ok(r) if r > 0.0 => r as f32,
        Ok(_) => return Outcome::err(RpcError::invalid_params("`radius` must be positive")),
        Err(e) => return Outcome::err(e),
    };
    let centre = match (opt_f64(params, "x"), opt_f64(params, "z")) {
        (Ok(Some(x)), Ok(Some(z))) => [x as f32, z as f32],
        (Ok(None), Ok(None)) => camera_xz(world).unwrap_or([0.0, 0.0]),
        (Err(e), _) | (_, Err(e)) => return Outcome::err(e),
        _ => {
            return Outcome::err(RpcError::invalid_params(
                "give both `x` and `z`, or neither",
            ))
        }
    };
    let q = match run_query(
        params,
        TraceQuery {
            near: Some((centre, radius)),
            ..TraceQuery::default()
        },
    ) {
        Ok(q) => q,
        Err(e) => return Outcome::err(e),
    };
    let full = opt_bool(params, "full").ok().flatten().unwrap_or(false);
    let records = st::query(&q);
    Outcome::ok(
        json!({"center": centre, "radius": radius, "count": records.len(), "records": rows(&records, full)}),
    )
}

/// `trace.export {path, file?, kind?, tile?, issues_only?}`: writes the matching in-memory
/// records as JSON Lines (the run file next to the logs holds everything).
fn export(world: &mut World, params: &Value) -> Outcome {
    if !st::enabled() {
        return Outcome::err(not_on());
    }
    let raw = match str_param(params, "path") {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let path = match world
        .resource::<PathPolicy>()
        .check_write(&PathBuf::from(raw))
    {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let tile = match tile_param(params) {
        Ok(t) => t,
        Err(e) => return Outcome::err(e),
    };
    let mut q = match run_query(
        params,
        TraceQuery {
            tile,
            ..TraceQuery::default()
        },
    ) {
        Ok(q) => q,
        Err(e) => return Outcome::err(e),
    };
    q.limit = opt_u64(params, "limit")
        .ok()
        .flatten()
        .map_or(usize::MAX / 2, |n| n as usize);
    let lines = st::export_lines(&q);
    let mut text = lines.join("\n");
    text.push('\n');
    match std::fs::write(&path, text) {
        Ok(()) => Outcome::ok(json!({"path": path.display().to_string(), "records": lines.len()})),
        Err(e) => Outcome::err(RpcError::new(code::FAILED, format!("write failed: {e}"))),
    }
}
