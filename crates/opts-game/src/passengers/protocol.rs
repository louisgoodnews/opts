//! The control protocol's `vehicle.passengers`.

use bevy::prelude::*;
use opts_bevy::bus_stops::BusStopMarker;
use opts_bevy::PlayerVehicle;

use super::PassengerRig;

/// Applies the request's side effects: the mass count override, forced boarding at an entry and
/// arriving at the stop. Fails when the player's vehicle has no passenger cabin.
fn apply_request(
    world: &mut World,
    params: &serde_json::Value,
) -> Result<(), opts_cli::protocol::RpcError> {
    use opts_cli::protocol::{code, RpcError};
    let spawn = params.get("spawn").and_then(serde_json::Value::as_u64);
    if let Some(n) = params.get("mass_count").and_then(serde_json::Value::as_i64) {
        world.insert_resource(super::PassengerCountOverride(
            usize::try_from(n).ok().map(|n| n.min(400)),
        ));
    }
    {
        let mut q0 = world.query_filtered::<&mut PassengerRig, With<PlayerVehicle>>();
        let Some(mut rig) = q0.iter_mut(world).next() else {
            return Err(RpcError::new(
                code::NOT_READY,
                "the player's vehicle has no passenger cabin (or there is no drive)",
            ));
        };
        if let Some(n) = spawn {
            rig.force_group = n.min(400) as u32;
            rig.force_entry = params
                .get("entry")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize;
        }
        if params
            .get("arrive")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            rig.sim.arrive_at_stop();
        }
    }
    Ok(())
}

/// The nearest bus stop: distance (m), id code, position and heading (deg).
type NearestStop = (f32, u64, Vec3, f32);

/// The `vehicle.passengers` reply: the cabin, the stops around and every passenger.
fn state_json(
    world: &World,
    rig: &PassengerRig,
    (stops_in_world, here, nearest): (usize, Option<Vec3>, Option<NearestStop>),
) -> serde_json::Value {
    let sim = &rig.sim;
    let forced = world
        .get_resource::<super::PassengerCountOverride>()
        .and_then(|f| f.0);
    let places: Vec<serde_json::Value> = (0..sim.cabin.places.len())
        .map(|i| serde_json::json!(sim.on_place(i)))
        .collect();
    serde_json::json!({
        "inside": sim.humans_inside(),
        "mass": {
            "source": if forced.is_some() { "override" } else { "passengers" },
            "count": forced.unwrap_or_else(|| sim.humans_inside()),
            "by_unit": sim.humans_inside_by_unit(rig.units.len().max(1)),
            "on": super::weight::mass_settings(world.get_resource::<crate::settings::GameSettings>()).0,
        },
        "approaching": rig.approaching.len(),
        "leaving": rig.leaving.len(),
        "at_stop": rig.stop.is_some(),
        "stops_served": rig.stops_served,
        "stops_in_world": stops_in_world,
        "vehicle_omsi_position": here.map(|h| [h.x, -h.z, h.y]),
        "nearest_stop": nearest.map(|(d, id, p, hd)| serde_json::json!({"distance_m": d, "id_code": id, "omsi_position": [p.x, -p.z, p.y], "heading_deg": hd})),
        "seats": sim.cabin.seat_count(),
        "standing_places": sim.cabin.standing_count(),
        "on_places": places,
        "entry_requests": sim.entry_requests(),
        "exit_requests": sim.exit_requests(),
        "revenue": sim.revenue,
        "tickets_in_pack": sim.tickets.tickets.len(),
        "passengers": sim.passengers().iter().map(|p| serde_json::json!({
            "id": p.id, "age": p.age, "phase": format!("{:?}", p.phase),
            "ticket": p.ticket, "stamped": p.stamped, "stops_left": p.stops_left,
            "unit": p.unit, "omsi_position": [p.position.x, p.position.y, p.position.z],
            // The drawn figure's root (its feet) in the unit's frame, OMSI axes.
            "figure_root": rig.actors.get(&p.id).map(|a| [a.local.x, -a.local.z, a.local.y]),
        })).collect::<Vec<_>>(),
        "inside_per_unit": (0..rig.units.len()).map(|u| sim.passengers().iter()
            .filter(|p| p.unit == u && !matches!(p.phase, opts_sim::passengers::Phase::WaitingOutside { .. }))
            .count()).collect::<Vec<_>>(),
    })
}

/// `vehicle.passengers {spawn?}`: reads the passengers of the player's vehicle, or sends `spawn`
/// people to its doors now.
pub fn command(
    world: &mut World,
    params: &serde_json::Value,
) -> opts_bevy::control::state::Outcome {
    use opts_bevy::control::state::Outcome;
    use opts_cli::protocol::{code, RpcError};
    if let Err(e) = apply_request(world, params) {
        return Outcome::err(e);
    }
    let spots: Vec<(Vec3, u64, f32)> = {
        let mut m = world.query::<(&GlobalTransform, &BusStopMarker)>();
        m.iter(world)
            .map(|(g, b)| {
                // Bevy forward is -Z; OMSI headings run clockwise from north (+Y, Bevy -Z).
                let f = g.rotation() * Vec3::NEG_Z;
                (g.translation(), b.id_code, f.x.atan2(-f.z).to_degrees())
            })
            .collect()
    };
    let here = {
        let mut v =
            world.query_filtered::<&GlobalTransform, (With<PlayerVehicle>, With<PassengerRig>)>();
        v.iter(world).next().map(GlobalTransform::translation)
    };
    let mut q = world.query_filtered::<&mut PassengerRig, With<PlayerVehicle>>();
    let Some(rig) = q.iter(world).next() else {
        return Outcome::err(RpcError::new(code::NOT_READY, "no passenger cabin"));
    };
    let nearest = here.and_then(|h| {
        spots
            .iter()
            .map(|(p, id, hd)| (Vec2::new(p.x - h.x, p.z - h.z).length(), *id, *p, *hd))
            .min_by(|a, b| a.0.total_cmp(&b.0))
    });
    Outcome::ok(state_json(world, rig, (spots.len(), here, nearest)))
}

/// Registers the protocol command.
pub fn register(registry: &mut opts_bevy::control::state::ControlRegistry) {
    registry.register("vehicle.passengers", command);
}
