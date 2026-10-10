//! `camera.set_mode`. Scene-level commands that need game state (`scene.load_map`, `vehicle.set`,
//! `camera.teleport`, …) are registered by `opts-game`.

use bevy::ecs::world::World;
use opts_cli::protocol::{code, RpcError};
use serde_json::{json, Value};

use crate::camera::{CameraAvailability, CameraMode, CameraState};
use crate::control::params::{opt_u64, str_param};
use crate::control::state::{ControlRegistry, ControlState, Outcome};

/// Registers the scene methods owned by `opts-bevy`.
pub fn register(r: &mut ControlRegistry) {
    r.register("camera.set_mode", set_mode);
}

fn parse_mode(text: &str) -> Option<CameraMode> {
    match text.to_ascii_lowercase().as_str() {
        "1" | "ego" => Some(CameraMode::Ego),
        "2" | "cockpit" => Some(CameraMode::Cockpit),
        "3" | "external" => Some(CameraMode::External),
        "4" | "pax" | "passenger" => Some(CameraMode::Passenger),
        "5" | "free" | "freeroam" => Some(CameraMode::FreeRoam),
        _ => None,
    }
}

/// Switches the mode like the number keys do: the key is injected, so availability checks and the
/// side effects of entering a mode are the same. The answer follows two frames later.
fn set_mode(world: &mut World, params: &Value) -> Outcome {
    let (mode, sub) = match (str_param(params, "mode"), opt_u64(params, "sub")) {
        (Ok(text), Ok(sub)) => match parse_mode(text) {
            Some(m) => (m, sub),
            None => {
                return Outcome::err(RpcError::invalid_params(
                    "`mode` must be ego, cockpit, external, pax or free (or 1-5)",
                ))
            }
        },
        (Err(e), _) | (_, Err(e)) => return Outcome::err(e),
    };
    if world.get_resource::<CameraState>().is_none() {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no camera system in this state",
        ));
    }
    if !world
        .get_resource::<CameraAvailability>()
        .is_some_and(|a| a.allows(mode))
    {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            format!("camera mode `{}` is not available now", mode.label()),
        ));
    }
    let digit = format!("{}", mode.key_number());
    let pressed = crate::control::channel::dispatch(world, "input.key", &json!({ "key": digit }));
    if let Outcome::Done(Err(e)) = pressed {
        return Outcome::err(e);
    }
    let start = world.resource::<ControlState>().frame;
    Outcome::Wait(Box::new(move |world| {
        if world.resource::<ControlState>().frame < start + 2 {
            return None;
        }
        let mut state = world.resource_mut::<CameraState>();
        if let Some(s) = sub {
            state.sub_camera_index = s as usize;
        }
        Some(Ok(
            json!({ "mode": state.mode.label(), "sub_index": state.sub_camera_index }),
        ))
    }))
}
