//! Trace queries of the control protocol: picking an object and running a query over the trace.

use super::*;

/// The tile a Bevy ground position lies on.
fn tile_at(world: &World, x: f32, z: f32) -> Option<(i32, i32)> {
    let stats = world.get_resource::<WorldStats>()?;
    Some(stats.grid.key_at(Vec2::new(x, -z), stats.start_tile))
}

/// `trace.pick {x?, y?, crosshair?, limit?}`: what is under a screen pixel (as in a screenshot)
/// or under the crosshair (the centre of the view; the default).
pub(super) fn pick(world: &mut World, params: &Value) -> Outcome {
    if !st::enabled() {
        return Outcome::err(not_on());
    }
    let point = match (opt_f64(params, "x"), opt_f64(params, "y")) {
        (Ok(Some(x)), Ok(Some(y))) => Some((x as f32, y as f32)),
        (Ok(None), Ok(None)) => None,
        (Err(e), _) | (_, Err(e)) => return Outcome::err(e),
        _ => {
            return Outcome::err(RpcError::invalid_params(
                "give both `x` and `y`, or neither",
            ))
        }
    };
    let mut cams = world.query::<(&Camera, &GlobalTransform, &Projection)>();
    let Some((camera, gt, _)) = cams
        .iter(world)
        .filter(|(c, _, _)| c.is_active)
        .max_by_key(|(c, _, _)| c.order)
    else {
        return Outcome::err(RpcError::new(code::NOT_READY, "there is no active camera"));
    };
    let Some(size) = camera.physical_viewport_size() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "the camera has no viewport yet",
        ));
    };
    let physical = point.map_or(
        Vec2::new(size.x as f32 / 2.0, size.y as f32 / 2.0),
        |(x, y)| Vec2::new(x, y),
    );
    if physical.x < 0.0
        || physical.y < 0.0
        || physical.x > size.x as f32
        || physical.y > size.y as f32
    {
        return Outcome::err(RpcError::invalid_params(format!(
            "the point lies outside the {}x{} view",
            size.x, size.y
        )));
    }
    let logical = physical / camera.target_scaling_factor().unwrap_or(1.0);
    let ray = match camera.viewport_to_world(gt, logical) {
        Ok(r) => r,
        Err(e) => {
            return Outcome::err(RpcError::new(
                code::FAILED,
                format!("no ray for the point: {e}"),
            ))
        }
    };
    let (origin, dir) = (ray.origin, *ray.direction);
    let records = st::query(&TraceQuery {
        limit: usize::MAX / 2,
        ..TraceQuery::default()
    });
    let hits = st::pick::pick(&records, origin, dir);
    let limit = limit_param(params, 5);
    let candidates: Vec<Value> = hits
        .iter()
        .take(limit)
        .filter_map(|h| {
            let r = records.iter().find(|r| r.id == h.id)?;
            let mut row = summary_row(r);
            row["distance"] = json!(h.distance);
            Some(row)
        })
        .collect();
    let best = hits
        .first()
        .and_then(|h| records.iter().find(|r| r.id == h.id));
    // The ground point (plane y = 0, approximate on hilly terrain) and its tile.
    let ground = (dir.y < -1e-4).then(|| {
        let t = -origin.y / dir.y;
        let p = origin + dir * t.max(0.0);
        json!({"point": [p.x, p.y, p.z], "tile": tile_at(world, p.x, p.z).map(|t| [t.0, t.1])})
    });
    let mut out = json!({
        "pixel": [physical.x, physical.y],
        "view": [size.x, size.y],
        "ray": {"origin": [origin.x, origin.y, origin.z], "dir": [dir.x, dir.y, dir.z]},
        "hit": best.map(record_json),
        "parents": best.map_or_else(Vec::new, |r| r.parents.iter().map(|p| serde_json::to_value(p).unwrap_or(Value::Null)).collect::<Vec<_>>()),
        "candidates": candidates,
        "ground": ground,
    });
    if let Some(r) = best {
        // A parent that has a record of its own is returned in full.
        let parents: Vec<Value> = r
            .parents
            .iter()
            .map(|p| {
                let full = records.iter().find(|c| {
                    c.kind == p.kind
                        && c.tile == p.tile
                        && c.index == p.index
                        && p.index.is_some()
                        && c.list.as_deref()
                            == Some(if p.kind == "spline" {
                                "splines"
                            } else {
                                "objects"
                            })
                });
                full.map_or_else(
                    || serde_json::to_value(p).unwrap_or(Value::Null),
                    record_json,
                )
            })
            .collect();
        out["parents"] = Value::Array(parents);
    }
    if let Some(mut o) = world.get_resource_mut::<TraceOverlay>() {
        o.picked = best.map(|r| r.id);
    }
    Outcome::ok(out)
}

pub(super) fn run_query(params: &Value, mut q: TraceQuery) -> Result<TraceQuery, RpcError> {
    q.limit = limit_param(params, 200);
    q.file = opt_str(params, "file")?
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    q.kind = opt_str(params, "kind")?
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    q.stage = opt_str(params, "stage")?
        .filter(|s| !s.is_empty())
        .map(str::to_owned);
    q.issues_only = opt_bool(params, "issues_only")?.unwrap_or(false);
    Ok(q)
}

pub(super) fn rows(records: &[st::TraceRecord], full: bool) -> Vec<Value> {
    records
        .iter()
        .map(|r| if full { record_json(r) } else { summary_row(r) })
        .collect()
}
