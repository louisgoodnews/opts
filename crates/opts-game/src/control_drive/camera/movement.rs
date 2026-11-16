//! `camera.move`: moves the free-roam camera by a relative offset.

use super::*;

pub(in crate::control_drive) fn move_camera(world: &mut World, params: &Value) -> Outcome {
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
