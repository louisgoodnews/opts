//! `camera.look`: turns the camera by a yaw and pitch, or looks at a target point.

use super::*;

pub(in crate::control_drive) fn look(world: &mut World, params: &Value) -> Outcome {
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
