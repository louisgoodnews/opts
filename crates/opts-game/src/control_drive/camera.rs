//! Camera control of the drive protocol: placing, teleporting and resetting the vehicle and the camera.

use super::state::absolute_position;
use super::*;

fn place(
    world: &mut World,
    local: Vec2,
    heading_rad: Option<f32>,
    elevation: Option<f32>,
) -> Result<(), RpcError> {
    let mut q = world.query_filtered::<&mut VehicleController, With<PlayerVehicle>>();
    let Some(mut c) = q.iter_mut(world).next() else {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    };
    let heading = heading_rad.unwrap_or(c.sim.chassis.heading_rad);
    c.sim.teleport(local, heading);
    let ch = &mut c.sim.chassis;
    if let Some(z) = elevation {
        ch.elevation_z = z;
    }
    ch.local_velocity = Vec2::ZERO;
    ch.yaw_rate_rad_s = 0.0;
    ch.local_acceleration = Vec2::ZERO;
    c.render_prev = None;
    let mut r = world.resource_mut::<RemoteControls>();
    r.release_all();
    r.restart_odometer();
    Ok(())
}

pub(super) fn teleport(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let x = opts_bevy::control::params::f64_param(params, "x")?;
        let y = opts_bevy::control::params::f64_param(params, "y")?;
        let heading = opt_f64(params, "heading_deg")?;
        let (start, grid) = world
            .get_resource::<WorldStats>()
            .map(|s| (s.start_tile, s.grid))
            .ok_or_else(|| RpcError::new(code::NOT_READY, "no session has been started"))?;
        let local = grid
            .absolute_to_local(bevy::math::DVec2::new(x, y), start)
            .as_vec2();
        place(world, local, heading.map(|h| (h as f32).to_radians()), None)?;
        Ok(json!({ "ok": true, "x": x, "y": y, "heading_deg": heading }))
    })();
    res.into()
}

pub(super) fn reset(world: &mut World, _: &Value) -> Outcome {
    let res = (|| {
        let pose = world
            .get_resource::<RemoteControls>()
            .and_then(|r| r.start_pose)
            .ok_or_else(|| {
                RpcError::new(
                    code::NOT_READY,
                    "there is no player vehicle (no drive is running)",
                )
            })?;
        place(world, pose.0, Some(pose.1), Some(pose.2))?;
        let (x, y) = absolute_position(world, pose.0);
        Ok(json!({ "ok": true, "x": x, "y": y, "heading_deg": pose.1.to_degrees() }))
    })();
    res.into()
}

pub(super) fn reset_camera(world: &mut World, _: &Value) -> Outcome {
    if world.get_resource::<CameraState>().is_none() {
        return Outcome::err(RpcError::new(code::NOT_READY, "there is no camera"));
    }
    if modal_open(world) {
        return Outcome::err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: camera input is blocked",
        ));
    }
    let mut camera = world.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
    let mut tf = camera.iter_mut(world).next().map(|t| *t);
    let mut state = world.resource::<CameraState>().clone();
    let mut viewpoint = world
        .get_resource::<opts_bevy::ViewpointState>()
        .cloned()
        .unwrap_or_default();
    let done = opts_bevy::camera::reset::reset_view(&mut state, tf.as_mut(), &mut viewpoint);
    *world.resource_mut::<CameraState>() = state;
    if let Some(mut vp) = world.get_resource_mut::<opts_bevy::ViewpointState>() {
        *vp = viewpoint;
    }
    if let Some(tf) = tf {
        for mut t in camera.iter_mut(world) {
            *t = tf;
        }
    }
    Outcome::ok(json!({
        "mode": format!("{:?}", done.mode).to_lowercase(),
        "sub_camera": done.sub_camera,
    }))
}

mod look;
mod movement;
pub(super) use look::look;
pub(super) use movement::move_camera;
