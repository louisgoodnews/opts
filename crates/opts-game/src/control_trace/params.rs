//! Parameter helpers and filter/status JSON of the `trace.*` methods.

use super::*;

pub(super) fn not_on() -> RpcError {
    RpcError::new(
        code::NOT_READY,
        "the scene trace is off; switch it on first (`trace.set mode=full`, or the setting, or --trace-scene) and load the map again",
    )
}

pub(super) fn limit_param(params: &Value, default: u64) -> usize {
    opt_u64(params, "limit")
        .ok()
        .flatten()
        .unwrap_or(default)
        .clamp(1, 5000) as usize
}

pub(super) fn tile_param(params: &Value) -> Result<Option<(i32, i32)>, RpcError> {
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

pub(super) fn camera_xz(world: &mut World) -> Option<[f32; 2]> {
    let mut q = world.query_filtered::<&GlobalTransform, With<opts_bevy::MainCamera>>();
    q.iter(world)
        .next()
        .map(|t| [t.translation().x, t.translation().z])
}

pub(super) fn parse_filters(
    world: &mut World,
    params: &Value,
    base: Filters,
) -> Result<Filters, RpcError> {
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

pub(super) fn filters_json(f: &Filters) -> Value {
    json!({
        "tiles": f.tiles, "kinds": f.kinds, "file": f.file,
        "radius": f.radius.map(|r| json!({"center": r.center, "meters": r.meters})),
    })
}

pub(super) fn status_json() -> Value {
    let mut v = serde_json::to_value(st::status()).unwrap_or(Value::Null);
    if let Value::Object(m) = &mut v {
        m.insert("filters".into(), filters_json(&st::filters()));
        if m.get("mode").and_then(Value::as_str) == Some("") {
            m.insert("mode".into(), json!("off"));
        }
    }
    v
}
