//! Driving and camera commands of the control protocol: `drive.*`, `vehicle.teleport`,
//! `vehicle.reset`, `camera.look`, `camera.move` and `snapshot`.
//!
//! The controls (`drive.set`, `drive.trigger`, `drive.release`) feed
//! [`opts_bevy::vehicle_remote::RemoteControls`], which the vehicle input system merges under the
//! same run conditions as the keyboard. The handlers also test those conditions when the command
//! arrives and answer `1009` (gated) instead of queueing something that would never run.

use bevy::input::keyboard::{Key, KeyCode, KeyboardInput};
use bevy::input::mouse::{MouseButton, MouseButtonInput, MouseMotion};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::time::{Real, Time, Virtual};
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::handlers::input::primary_window_or_placeholder;
use opts_bevy::control::keys::parse_key;
use opts_bevy::control::params::{opt_f64, opt_u64, str_param};
use opts_bevy::control::state::{ControlRegistry, ControlState, Outcome, Release};
use opts_bevy::vehicle_remote::{raise_trigger, Hold, ParkingRequest, RemoteControls};
use opts_bevy::{CameraMode, CameraState, PlayerVehicle, ScriptKeyBindings, VehicleController};
use opts_cli::protocol::{code, RpcError};
use opts_sim::vehicle::inputs::GearPosition;
use serde_json::{json, Value};

use crate::in_game::omsi_world::WorldStats;

/// Registers the methods.
pub fn register(r: &mut ControlRegistry) {
    r.register("drive.controls", controls);
    r.register("drive.set", set);
    r.register("drive.release", release);
    r.register("drive.trigger", trigger);
    r.register("drive.state", drive_state);
    r.register("vehicle.teleport", teleport);
    r.register("vehicle.reset", reset);
    r.register("camera.look", look);
    r.register("camera.reset", reset_camera);
    r.register("camera.move", move_camera);
    r.register("snapshot", snapshot);
}

/// Short names for the standard OMSI trigger names, used when the vehicle binds the standard one.
pub const TRIGGER_ALIASES: &[(&str, &str)] = &[
    ("door_front", "bus_doorfront0"),
    ("door_middle", "bus_doormiddle0"),
    ("door_rear", "bus_doorrear0"),
    ("indicator_left", "blinker_left_set"),
    ("indicator_right", "blinker_right_set"),
    ("indicator_off", "blinker_off"),
    ("lights", "kw_scheinwerfer_toggle"),
];

fn resolve_trigger(bindings: &ScriptKeyBindings, name: &str) -> String {
    if bindings.0.iter().any(|b| b.trigger == name) {
        return name.to_string();
    }
    TRIGGER_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or_else(|| name.to_string(), |(_, real)| (*real).to_string())
}

// ---- gates ----------------------------------------------------------------------------------

fn modal_open(world: &World) -> bool {
    world
        .get_resource::<opts_bevy::reporting::state::ModalActive>()
        .is_some_and(|m| m.0)
        || world
            .get_resource::<opts_bevy::reporting::state::MenuModalActive>()
            .is_some_and(|m| m.0)
}

fn has_player(world: &mut World) -> bool {
    world
        .query_filtered::<(), With<PlayerVehicle>>()
        .iter(world)
        .next()
        .is_some()
}

/// The conditions under which the vehicle input systems run, checked now.
fn vehicle_gate(world: &mut World) -> Result<(), RpcError> {
    if !has_player(world) {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: vehicle input is blocked, exactly like a key press",
        ));
    }
    if world
        .get_resource::<CameraState>()
        .is_some_and(|c| c.mode == CameraMode::FreeRoam)
    {
        return Err(RpcError::new(
            code::GATED,
            "the camera is in free roam: the vehicle takes no input (`camera.set_mode external` first)",
        ));
    }
    Ok(())
}

fn trigger_gate(world: &mut World) -> Result<(), RpcError> {
    if !has_player(world) {
        return Err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    }
    if modal_open(world) {
        return Err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: vehicle triggers are blocked, exactly like a key press",
        ));
    }
    Ok(())
}

fn gear_name(g: GearPosition) -> &'static str {
    match g {
        GearPosition::Drive => "drive",
        GearPosition::Neutral => "neutral",
        GearPosition::Reverse => "reverse",
        #[allow(unreachable_patterns)]
        _ => "other",
    }
}

// ---- drive.controls ---------------------------------------------------------------------------

fn controls(world: &mut World, _: &Value) -> Outcome {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    let Some(c) = q.iter(world).next() else {
        return Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        ));
    };
    let i = c.inputs();
    let list = json!([
        { "name": "throttle", "kind": "analog", "range": [0.0, 1.0], "value": i.throttle() },
        { "name": "brake", "kind": "analog", "range": [0.0, 1.0], "value": i.service_brake() },
        { "name": "steer", "kind": "analog", "range": [-1.0, 1.0], "value": i.steering() },
        { "name": "gear", "kind": "enum", "values": ["drive", "neutral", "reverse"], "value": gear_name(i.gear()) },
        { "name": "parking_brake", "kind": "switch", "values": [true, false, "toggle"], "value": i.is_parking_brake_engaged() },
    ]);
    let bindings = world.resource::<ScriptKeyBindings>();
    let triggers: Vec<Value> = bindings
        .0
        .iter()
        .map(|b| {
            let alias = TRIGGER_ALIASES
                .iter()
                .find(|(_, real)| *real == b.trigger)
                .map(|(a, _)| *a);
            json!({ "name": b.trigger, "alias": alias, "key": format!("{:?}", b.key),
                    "shift": b.shift, "ctrl": b.ctrl, "held": b.held })
        })
        .collect();
    let held = world.resource::<RemoteControls>();
    Outcome::ok(json!({
        "controls": list,
        "triggers": triggers,
        "held": {
            "throttle": held.throttle.map(|h| h.value),
            "brake": held.brake.map(|h| h.value),
            "steer": held.steer.map(|h| h.value),
        },
    }))
}

// ---- drive.set / release / trigger ----------------------------------------------------------

fn number_value(params: &Value, name: &str, lo: f64, hi: f64) -> Result<f32, RpcError> {
    let v = params
        .get("value")
        .and_then(Value::as_f64)
        .ok_or_else(|| RpcError::invalid_params(format!("`value` of `{name}` must be a number")))?;
    if !(lo..=hi).contains(&v) {
        return Err(RpcError::invalid_params(format!(
            "`value` of `{name}` must be between {lo} and {hi}"
        )));
    }
    Ok(v as f32)
}

fn set(world: &mut World, params: &Value) -> Outcome {
    match set_inner(world, params) {
        Ok(v) => Outcome::ok(v),
        Err(e) => Outcome::err(e),
    }
}

fn set_inner(world: &mut World, params: &Value) -> Result<Value, RpcError> {
    let name = str_param(params, "control")?.to_ascii_lowercase();
    let hold_ms = opt_u64(params, "hold_ms")?;
    let remaining = hold_ms.map(|ms| ms as f32 / 1000.0);
    let hold = |v: f32| Hold {
        value: v,
        remaining,
    };
    match name.as_str() {
        "throttle" | "brake" | "steer" => {
            let (lo, hi) = if name == "steer" {
                (-1.0, 1.0)
            } else {
                (0.0, 1.0)
            };
            let v = number_value(params, &name, lo, hi)?;
            vehicle_gate(world)?;
            let mut r = world.resource_mut::<RemoteControls>();
            match name.as_str() {
                "throttle" => r.throttle = Some(hold(v)),
                "brake" => r.brake = Some(hold(v)),
                _ => r.steer = Some(hold(v)),
            }
            Ok(json!({ "ok": true, "control": name, "value": v, "hold_ms": hold_ms }))
        }
        "gear" => {
            let text = params
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let gear = match text.as_str() {
                "drive" | "d" => GearPosition::Drive,
                "neutral" | "n" => GearPosition::Neutral,
                "reverse" | "r" => GearPosition::Reverse,
                _ => {
                    return Err(RpcError::invalid_params(
                        "`value` of `gear` is drive, neutral or reverse",
                    )
                    .with_data(json!({ "valid": ["drive", "neutral", "reverse"] })))
                }
            };
            vehicle_gate(world)?;
            world.resource_mut::<RemoteControls>().gear = Some(gear);
            Ok(json!({ "ok": true, "control": "gear", "value": gear_name(gear) }))
        }
        "parking_brake" => {
            let request = match params.get("value") {
                Some(Value::Bool(b)) => ParkingRequest::Set(*b),
                Some(Value::String(s)) if s == "toggle" => ParkingRequest::Toggle,
                Some(Value::String(s)) if s == "on" => ParkingRequest::Set(true),
                Some(Value::String(s)) if s == "off" => ParkingRequest::Set(false),
                _ => {
                    return Err(RpcError::invalid_params(
                        "`value` of `parking_brake` is true, false or \"toggle\"",
                    ))
                }
            };
            vehicle_gate(world)?;
            world.resource_mut::<RemoteControls>().parking = Some(request);
            Ok(json!({ "ok": true, "control": "parking_brake", "value": params["value"] }))
        }
        _ => {
            // Any other name is a script trigger (doors, lights, indicators, horn).
            let on = params.get("value").and_then(Value::as_bool).unwrap_or(true);
            raise(world, &name, on, hold_ms)
        }
    }
}

fn raise(world: &mut World, name: &str, on: bool, hold_ms: Option<u64>) -> Result<Value, RpcError> {
    trigger_gate(world)?;
    let real = resolve_trigger(world.resource::<ScriptKeyBindings>(), name);
    let held = world
        .resource::<ScriptKeyBindings>()
        .0
        .iter()
        .any(|b| b.trigger == real && b.held);
    let to_raise = if on {
        real.clone()
    } else {
        format!("{real}_off")
    };
    let handled = raise_trigger(world, &to_raise)
        .ok_or_else(|| RpcError::new(code::NOT_READY, "the player vehicle has no scripts"))?;
    let mut held_ms = None;
    if on && held {
        let ms = hold_ms.unwrap_or(200);
        world
            .resource_mut::<RemoteControls>()
            .release_triggers
            .push((format!("{real}_off"), ms as f32 / 1000.0));
        held_ms = Some(ms);
    }
    Ok(
        json!({ "ok": true, "name": real, "requested": name, "raised": to_raise, "handled": handled, "held_ms": held_ms }),
    )
}

fn trigger(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let name = str_param(params, "name")?.to_string();
        let hold_ms = opt_u64(params, "hold_ms")?;
        raise(world, &name, true, hold_ms)
    })();
    res.into()
}

fn release(world: &mut World, params: &Value) -> Outcome {
    let res = (|| {
        let which = params
            .get("control")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase);
        let mut r = world.resource_mut::<RemoteControls>();
        match which.as_deref() {
            None | Some("all") => r.release_all(),
            Some("throttle") => r.throttle = None,
            Some("brake") => r.brake = None,
            Some("steer") => r.steer = None,
            Some("gear") => r.gear = None,
            Some("parking_brake") => r.parking = None,
            Some(other) => return Err(RpcError::invalid_params(format!(
                "`{other}` is not a held control"
            ))
            .with_data(
                json!({ "valid": ["throttle", "brake", "steer", "gear", "parking_brake", "all"] }),
            )),
        }
        Ok(json!({ "ok": true, "released": which.unwrap_or_else(|| "all".into()) }))
    })();
    res.into()
}

// ---- drive.state ------------------------------------------------------------------------------

fn absolute_position(world: &World, local: Vec2) -> (f64, f64) {
    let (start, grid) = world
        .get_resource::<WorldStats>()
        .map_or(((0, 0), Default::default()), |s| (s.start_tile, s.grid));
    let abs = grid.local_to_absolute(local.as_dvec2(), start);
    (abs.x, abs.y)
}

/// The drive state as JSON; `None` without a player vehicle.
pub fn drive_state_json(world: &mut World) -> Option<Value> {
    let mut q = world.query_filtered::<&VehicleController, With<PlayerVehicle>>();
    let c = q.iter(world).next()?.clone();
    let ch = &c.sim.chassis;
    let i = c.inputs();
    let (x, y) = absolute_position(world, ch.position);
    let signed = ch.forward_speed_km_h();
    let virt = world.resource::<Time<Virtual>>();
    let (paused, scale, sim_time) = (
        virt.is_paused(),
        virt.relative_speed(),
        virt.elapsed_secs_f64(),
    );
    let distance = world
        .get_resource::<RemoteControls>()
        .map_or(0.0, |r| r.odometer_m);
    let alert = world
        .get_resource::<crate::hud::alerts::AlertQueue>()
        .and_then(|q| {
            q.visible()
                .map(|a| json!({ "id": a.id, "severity": a.severity.id() }))
        });
    let camera = world.get_resource::<CameraState>().map(|c| c.mode.label());
    Some(json!({
        "speed_kmh": signed.abs(),
        "signed_speed_kmh": signed,
        "x": x, "y": y, "z": ch.elevation_z,
        "heading_deg": ch.heading_rad.to_degrees(),
        "gear": gear_name(i.gear()),
        "throttle": i.throttle(),
        "brake": i.service_brake(),
        "steer": i.steering(),
        "parking_brake": i.is_parking_brake_engaged(),
        "rpm": crate::hud::values::RPM_PROVIDED.then(|| c.engine_rpm()),
        "distance_m": distance,
        "camera": camera,
        "paused": paused,
        "time_scale": scale,
        "sim_time": sim_time,
        "alert": alert,
        // The timetable is not implemented yet; both are reported as unavailable, not guessed.
        "route": null,
        "stops": null,
    }))
}

fn drive_state(world: &mut World, _: &Value) -> Outcome {
    match drive_state_json(world) {
        Some(v) => Outcome::ok(v),
        None => Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        )),
    }
}

// ---- vehicle.teleport / reset -------------------------------------------------------------------

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

fn teleport(world: &mut World, params: &Value) -> Outcome {
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

fn reset(world: &mut World, _: &Value) -> Outcome {
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

// ---- camera.reset ---------------------------------------------------------------------------

fn reset_camera(world: &mut World, _: &Value) -> Outcome {
    if world.get_resource::<CameraState>().is_none() {
        return Outcome::err(RpcError::new(code::NOT_READY, "there is no camera"));
    }
    if modal_open(world) {
        return Outcome::err(RpcError::new(
            code::GATED,
            "a dialog or the vehicle menu is open: camera input is blocked",
        ));
    }
    let mut camera = world.query_filtered::<&mut Transform, With<Camera3d>>();
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

// ---- camera.look / camera.move -----------------------------------------------------------------------

fn look(world: &mut World, params: &Value) -> Outcome {
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
        let mut cameras = world.query_filtered::<&mut Transform, With<Camera3d>>();
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

fn move_camera(world: &mut World, params: &Value) -> Outcome {
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

// ---- snapshot --------------------------------------------------------------------------------------

fn result_of(outcome: Outcome) -> Value {
    match outcome {
        Outcome::Done(Ok(v)) => v,
        Outcome::Done(Err(e)) => json!({ "error": e }),
        Outcome::Wait(_) => json!({ "error": "not immediate" }),
    }
}

fn snapshot(world: &mut World, params: &Value) -> Outcome {
    let parsed = (|| -> Result<(Option<String>, bool, usize), RpcError> {
        let path = params
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string);
        let shot = opts_bevy::control::params::opt_bool(params, "screenshot")?.unwrap_or(false);
        let n = opt_u64(params, "warnings")?.unwrap_or(20) as usize;
        Ok((path, shot, n))
    })();
    let (path, want_shot, n_warn) = match parsed {
        Ok(p) => p,
        Err(e) => return Outcome::err(e),
    };
    let json_path = match path.as_deref() {
        Some(p) => match world
            .resource::<opts_bevy::control::PathPolicy>()
            .check_write(std::path::Path::new(p))
        {
            Ok(p) => Some(p),
            Err(e) => return Outcome::err(e),
        },
        None => None,
    };
    let mut doc = serde_json::Map::new();
    doc.insert(
        "state".into(),
        result_of(dispatch(world, "state", &Value::Null)),
    );
    doc.insert("screen".into(), crate::control_ui::screen_json(world));
    if let Some(d) = drive_state_json(world) {
        doc.insert("drive".into(), d);
    }
    if world
        .get_resource::<crate::hud::build::ActiveProfile>()
        .is_some()
    {
        doc.insert(
            "hud".into(),
            result_of(dispatch(world, "hud.state", &Value::Null)),
        );
    }
    doc.insert(
        "camera".into(),
        result_of(dispatch(world, "camera.get", &Value::Null)),
    );
    doc.insert(
        "sim".into(),
        result_of(dispatch(world, "sim.get", &Value::Null)),
    );
    let warnings: Vec<Value> = opts_logging::recent_logs(
        n_warn,
        &opts_logging::LogQuery {
            min_level: Some(tracing::Level::WARN),
            ..Default::default()
        },
    )
    .into_iter()
    .map(|r| json!({ "ts": r.ts, "level": r.level, "target": r.target, "message": r.message }))
    .collect();
    doc.insert("warnings".into(), Value::Array(warnings));
    let finish = move |doc: serde_json::Map<String, Value>| -> Result<Value, RpcError> {
        let value = Value::Object(doc);
        if let Some(p) = &json_path {
            let text = serde_json::to_string_pretty(&value).unwrap_or_default();
            std::fs::write(p, text).map_err(|e| {
                RpcError::new(code::FAILED, format!("cannot write {}: {e}", p.display()))
            })?;
        }
        Ok(value)
    };
    if !want_shot {
        return Outcome::from(finish(doc));
    }
    match dispatch(world, "screenshot", &Value::Null) {
        Outcome::Wait(mut waiter) => Outcome::Wait(Box::new(move |world| {
            let shot = waiter(world)?;
            let mut doc = std::mem::take(&mut doc);
            match shot {
                Ok(s) => doc.insert("screenshot".into(), s),
                Err(e) => doc.insert("screenshot".into(), json!({ "error": e })),
            };
            Some(finish(doc))
        })),
        Outcome::Done(Err(e)) => {
            doc.insert("screenshot".into(), json!({ "error": e }));
            Outcome::from(finish(doc))
        }
        Outcome::Done(Ok(s)) => {
            doc.insert("screenshot".into(), s);
            Outcome::from(finish(doc))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_bevy::vehicle_triggers::ScriptKeyBinding;

    #[test]
    fn aliases_resolve_only_when_the_vehicle_does_not_bind_the_name() {
        let bindings = ScriptKeyBindings(vec![ScriptKeyBinding {
            trigger: "bus_doorfront0".into(),
            key: KeyCode::KeyH,
            shift: false,
            ctrl: false,
            held: false,
        }]);
        assert_eq!(resolve_trigger(&bindings, "door_front"), "bus_doorfront0");
        assert_eq!(
            resolve_trigger(&bindings, "bus_doorfront0"),
            "bus_doorfront0"
        );
        assert_eq!(
            resolve_trigger(&bindings, "my_own_trigger"),
            "my_own_trigger"
        );
    }

    #[test]
    fn numeric_values_are_range_checked() {
        assert!(number_value(&json!({"value": 0.5}), "throttle", 0.0, 1.0).is_ok());
        assert!(number_value(&json!({"value": 1.5}), "throttle", 0.0, 1.0).is_err());
        assert!(number_value(&json!({"value": "x"}), "steer", -1.0, 1.0).is_err());
        assert_eq!(
            number_value(&json!({"value": 2.0}), "steer", -1.0, 1.0)
                .unwrap_err()
                .code,
            code::INVALID_PARAMS
        );
    }
}
