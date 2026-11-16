//! The drive state of the protocol: the absolute position, the drive-state JSON and the snapshot of the vehicle.

use super::*;

pub(super) fn absolute_position(world: &World, local: Vec2) -> (f64, f64) {
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
    let mouse = world.get_resource::<opts_bevy::MouseDriving>().map(
        |m| json!({ "enabled": m.enabled, "on": m.on, "freed": m.freed, "steering": m.steering }),
    );
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
        "mouse_driving": mouse,
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

pub(super) fn drive_state(world: &mut World, _: &Value) -> Outcome {
    match drive_state_json(world) {
        Some(v) => Outcome::ok(v),
        None => Outcome::err(RpcError::new(
            code::NOT_READY,
            "there is no player vehicle (no drive is running)",
        )),
    }
}

mod snapshot;
pub(super) use snapshot::snapshot;
