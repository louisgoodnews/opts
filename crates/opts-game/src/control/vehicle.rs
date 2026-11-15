//! Vehicle commands of the control protocol: unload, vehicle.set and camera.teleport.


use super::*;


pub(super) fn read_path(world: &World, params: &Value, name: &str) -> Result<PathBuf, RpcError> {
    let raw = str_param(params, name)?;
    world
        .resource::<PathPolicy>()
        .check_read(&PathBuf::from(raw))
}

pub(super) fn unload(world: &mut World, _: &Value) -> Outcome {
    world
        .resource_mut::<NextState<AppState>>()
        .set(AppState::MainMenu);
    Outcome::ok(json!({ "ok": true }))
}

pub(super) fn set_vehicle(world: &mut World, params: &Value) -> Outcome {
    let file = match read_path(world, params, "path") {
        Ok(p) if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("bus")) => p,
        Ok(p) => {
            return Outcome::err(RpcError::invalid_params(format!(
                "{} is not a .bus file",
                p.display()
            )))
        }
        Err(e) => return Outcome::err(e),
    };
    world.resource_mut::<AppArgs>().0.vehicle_path = Some(file);
    if world.get_resource::<InGameSessionInfo>().is_none() {
        return Outcome::ok(json!({ "ok": true, "applied": "at the next session start" }));
    }
    restart_session(world)
}

/// OMSI world coordinates (metres, absolute) and compass heading → free-roam camera.
pub(super) fn teleport(world: &mut World, params: &Value) -> Outcome {
    let parsed = (|| -> Result<(f64, f64, f64, f64, f64), RpcError> {
        Ok((
            f64_param(params, "x")?,
            f64_param(params, "y")?,
            f64_param(params, "z")?,
            opt_f64(params, "yaw")?.unwrap_or(0.0),
            opt_f64(params, "pitch")?.unwrap_or(0.0),
        ))
    })();
    let (x, y, z, yaw, pitch) = match parsed {
        Ok(v) => v,
        Err(e) => return Outcome::err(e),
    };
    let Some((start, grid)) = world
        .get_resource::<WorldStats>()
        .map(|s| (s.start_tile, s.grid))
    else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "no session has been started",
        ));
    };
    let local = grid.absolute_to_local(bevy::math::DVec2::new(x, y), start);
    let local = Vec3::new(local.x as f32, local.y as f32, z as f32);
    let position = omsi_to_bevy_pos(local);
    let (yaw_rad, pitch_rad) = ((-yaw).to_radians() as f32, pitch.to_radians() as f32);
    let switched = dispatch(world, "camera.set_mode", &json!({ "mode": "free" }));
    let Outcome::Wait(mut switch) = switched else {
        return switched;
    };
    Outcome::Wait(Box::new(move |world| {
        let result = switch(world)?;
        if let Err(e) = result {
            return Some(Err(e));
        }
        let rotation = Quat::from_euler(EulerRot::YXZ, yaw_rad, pitch_rad, 0.0);
        let mut cameras = world.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
        for mut t in cameras.iter_mut(world) {
            t.translation = position;
            t.rotation = rotation;
        }
        let mut cam = world.resource_mut::<CameraState>();
        cam.free_roam_yaw = yaw_rad;
        cam.free_roam_pitch = pitch_rad;
        Some(Ok(
            json!({ "translation": [position.x, position.y, position.z] }),
        ))
    }))
}
