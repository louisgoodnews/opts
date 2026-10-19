//! Camera control of the drive protocol: placing, teleporting and resetting the vehicle and the camera, looking and moving the camera.

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

pub(super) fn look(world: &mut World, params: &Value) -> Outcome {
    match look_inner(world, params) {
        Ok(o) => o,
        Err(e) => Outcome::err(e),
    }
}

fn look_inner(world: &mut World, params: &Value) -> Result<Outcome, RpcError> {
    let (yaw, pitch) = (opt_f64(params, "yaw")?, opt_f64(params, "pitch")?);
    let (dx, dy) = (opt_f64(params, "dx")?, opt_f64(params, "dy")?);
    let mode = world
        .get_resource::<CameraState>()
        .map(|c| c.mode)
        .ok_or_else(|| RpcError::new(code::NOT_READY, "there is no camera"))?;
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: camera input is blocked",
        ));
    }
    if yaw.is_some() || pitch.is_some() {
        if mode != CameraMode::FreeRoam {
            return Err(RpcError::new(
                code::NOT_READY,
                "`yaw`/`pitch` set the free camera; use `dx`/`dy` in this mode",
            ));
        }
        let mut cam = world.resource_mut::<CameraState>();
        if let Some(y) = yaw {
            cam.free_roam_yaw = (-y).to_radians() as f32;
        }
        if let Some(p) = pitch {
            cam.free_roam_pitch = p.to_radians() as f32;
        }
        let rotation = Quat::from_euler(EulerRot::YXZ, cam.free_roam_yaw, cam.free_roam_pitch, 0.0);
        let (y, p) = (cam.free_roam_yaw, cam.free_roam_pitch);
        let mut cameras = world.query_filtered::<&mut Transform, With<opts_bevy::MainCamera>>();
        for mut t in cameras.iter_mut(world) {
            t.rotation = rotation;
        }
        return Ok(Outcome::ok(
            json!({ "mode": mode.label(), "yaw_deg": -y.to_degrees(), "pitch_deg": p.to_degrees() }),
        ));
    }
    let (dx, dy) = (dx.unwrap_or(0.0) as f32, dy.unwrap_or(0.0) as f32);
    if dx == 0.0 && dy == 0.0 {
        return Err(RpcError::invalid_params("give `yaw`/`pitch` or `dx`/`dy`"));
    }
    // The button a person holds while moving the mouse in this mode.
    let button = match mode {
        CameraMode::Cockpit | CameraMode::Passenger => Some(MouseButton::Right),
        CameraMode::External => Some(MouseButton::Left),
        _ => None,
    };
    let window = primary_window_or_placeholder(world);
    if let Some(b) = button {
        world.write_message(MouseButtonInput {
            button: b,
            state: ButtonState::Pressed,
            window,
        });
    }
    let mut frame = 0;
    Ok(Outcome::Wait(Box::new(move |world| {
        frame += 1;
        match frame {
            1 => {
                world.write_message(MouseMotion {
                    delta: Vec2::new(dx, dy),
                });
                None
            }
            2 => None,
            _ => {
                if let Some(b) = button {
                    world.write_message(MouseButtonInput {
                        button: b,
                        state: ButtonState::Released,
                        window,
                    });
                }
                Some(Ok(
                    json!({ "ok": true, "mode": mode.label(), "dx": dx, "dy": dy }),
                ))
            }
        }
    })))
}

pub(super) fn move_camera(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let (f, r, u) = (
            opt_f64(params, "forward")?.unwrap_or(0.0),
            opt_f64(params, "right")?.unwrap_or(0.0),
            opt_f64(params, "up")?.unwrap_or(0.0),
        );
        let ms = opt_u64(params, "duration_ms")?.unwrap_or(500).min(600_000);
        let mode = world.get_resource::<CameraState>().map(|c| c.mode);
        if mode != Some(CameraMode::FreeRoam) {
            return Err(RpcError::new(
                code::NOT_READY,
                "the camera is not in free roam (`camera.set_mode free`)",
            ));
        }
        if modal_open(world) {
            return Err(RpcError::new(
                code::GATED,
                "a dialog or the vehicle menu is open: camera input is blocked",
            ));
        }
        let window = primary_window_or_placeholder(world);
        let at = world.resource::<Time<Real>>().elapsed_secs_f64() + ms as f64 / 1000.0;
        let mut pressed = Vec::new();
        for (v, pos, neg) in [(f, "W", "S"), (r, "D", "A"), (u, "E", "Q")] {
            if v.abs() > f64::EPSILON {
                if let Some(k) = parse_key(if v > 0.0 { pos } else { neg }) {
                    pressed.push(k);
                }
            }
        }
        for (code, logical) in &pressed {
            world.write_message(KeyboardInput {
                key_code: *code,
                logical_key: logical.clone(),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        }
        let mut state = world.resource_mut::<ControlState>();
        for k in pressed {
            state.releases.push(Release {
                key: Some(k),
                mouse: None,
                at,
            });
        }
        let _: Option<Key> = None;
        let _ = KeyCode::KeyW;
        Ok(json!({ "ok": true, "mode": "Free", "duration_ms": ms }))
    })();
    res.into()
}
