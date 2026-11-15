//! The read commands of the scene trace: tile, find, region and export.


use super::*;


/// `trace.tile {tile, kind?, file?, stage?, issues_only?, full?, limit?}`: counts and records.
pub(super) fn tile(_: &mut World, params: &Value) -> Outcome {
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
pub(super) fn find(_: &mut World, params: &Value) -> Outcome {
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
pub(super) fn region(world: &mut World, params: &Value) -> Outcome {
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
pub(super) fn export(world: &mut World, params: &Value) -> Outcome {
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
