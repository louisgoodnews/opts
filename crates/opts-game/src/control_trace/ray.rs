//! The pick ray of the trace protocol: the world ray through a screen pixel (or the crosshair) of
//! the active camera, and the tile a ground point lies on.

use super::*;

/// The tile a Bevy ground position lies on.
pub(super) fn tile_at(world: &World, x: f32, z: f32) -> Option<(i32, i32)> {
    let stats = world.get_resource::<WorldStats>()?;
    Some(stats.grid.key_at(Vec2::new(x, -z), stats.start_tile))
}

/// The pixel (the middle of the view without `x` and `y`), the view size and the world ray through
/// it, from the active camera.
pub(super) fn pick_ray(world: &mut World, params: &Value) -> Result<(Vec2, UVec2, Vec3, Vec3), RpcError> {
    let point = match (opt_f64(params, "x"), opt_f64(params, "y")) {
        (Ok(Some(x)), Ok(Some(y))) => Some((x as f32, y as f32)),
        (Ok(None), Ok(None)) => None,
        (Err(e), _) | (_, Err(e)) => return Err(e),
        _ => {
            return Err(RpcError::invalid_params(
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
        return Err(RpcError::new(code::NOT_READY, "there is no active camera"));
    };
    let Some(size) = camera.physical_viewport_size() else {
        return Err(RpcError::new(
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
        return Err(RpcError::invalid_params(format!(
            "the point lies outside the {}x{} view",
            size.x, size.y
        )));
    }
    let logical = physical / camera.target_scaling_factor().unwrap_or(1.0);
    let ray = match camera.viewport_to_world(gt, logical) {
        Ok(r) => r,
        Err(e) => {
            return Err(RpcError::new(
                code::FAILED,
                format!("no ray for the point: {e}"),
            ))
        }
    };
    Ok((physical, size, ray.origin, *ray.direction))
}
