//! `camera.teleport`: moves the free-roam camera to OMSI world coordinates and a heading.

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::params::{f64_param, opt_f64};
use opts_bevy::control::state::Outcome;
use opts_bevy::coords::omsi_to_bevy_pos;
use opts_bevy::CameraState;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldStats;

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
