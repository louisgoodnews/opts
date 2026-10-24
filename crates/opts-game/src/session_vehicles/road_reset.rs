//! *Reset on the road*: puts a session vehicle on the nearest road lane under it, along the
//! lane, at rest. For a bus that has been turned, pushed off the road or got stuck; the spawn
//! point stays as it was (that is [`super::ops::respawn`]).

use bevy::prelude::*;
use opts_bevy::VehicleController;
use opts_traffic::LaneKind;

use super::ops::{local_to_abs, respawn_at, OpError};
use super::registry::{SessionVehicles, SpawnPoint, VehicleId};
use crate::in_game::omsi_world::TrafficState;

/// A lane counts as the vehicle's own when it runs within this angle of its heading (rad); the
/// nearest such lane is taken first.
const SAME_WAY_RAD: f64 = 45.0 * std::f64::consts::PI / 180.0;
/// … failing that, within this angle (a junction or a bend), then in any direction.
const NEAR_WAY_RAD: f64 = 100.0 * std::f64::consts::PI / 180.0;
/// Lanes farther than this from the vehicle are not considered (m).
const SEARCH_M: f64 = 60.0;

/// The nearest road lane place to `at` (session frame) for a vehicle heading `heading` (compass
/// radians): the point and the heading along the lane. A lane that runs the vehicle's way wins;
/// otherwise a lane the other way is taken and the vehicle keeps facing as it did.
#[must_use]
pub fn road_place(
    graph: &opts_traffic::LaneGraph,
    at: Vec2,
    heading: f32,
) -> Option<(bevy::math::DVec2, f64)> {
    let point = at.as_dvec2();
    let heading = f64::from(heading);
    let is_road = |l: &opts_traffic::graph::Lane| l.kind == LaneKind::Road;
    let same = graph.nearest_place(point, heading, SAME_WAY_RAD, SEARCH_M, is_road);
    let (lane, s, _) = same
        .or_else(|| graph.nearest_place(point, heading, std::f64::consts::PI, SEARCH_M, is_road))?;
    let lane = graph.lane(lane)?;
    let along = lane.geometry.heading_at(s);
    let mut diff = (along - heading).rem_euclid(std::f64::consts::TAU);
    if diff > std::f64::consts::PI {
        diff = std::f64::consts::TAU - diff;
    }
    // Keep the way the vehicle faced: against the lane, face against it.
    let facing = if diff <= std::f64::consts::FRAC_PI_2 {
        along
    } else {
        along + std::f64::consts::PI
    };
    Some((lane.geometry.point_at(s), facing))
}

/// Puts the vehicle on the nearest road lane under it, at rest.
///
/// # Errors
/// The vehicle does not exist or is not in the world, or no road lane is within 60 m.
pub fn reset_on_road(world: &mut World, id: VehicleId) -> Result<(), OpError> {
    let entity = world
        .resource::<SessionVehicles>()
        .get(id)
        .and_then(|r| r.entity)
        .ok_or_else(|| OpError::NotReady(format!("session vehicle {id} is not in the world")))?;
    let (position, heading, z) = world
        .get::<VehicleController>(entity)
        .map(|c| {
            (
                c.sim.chassis.position,
                c.sim.chassis.heading_rad,
                c.sim.chassis.elevation_z,
            )
        })
        .ok_or_else(|| OpError::NotReady(format!("session vehicle {id} has no simulation")))?;
    let place = world
        .get_resource::<TrafficState>()
        .and_then(|t| road_place(t.lane_graph(), position, heading))
        .ok_or_else(|| OpError::NotReady("no road lane within 60 m of the vehicle".into()))?;
    let local = Vec2::new(place.0.x as f32, place.0.y as f32);
    let (x, y) = local_to_abs(world, local);
    respawn_at(
        world,
        id,
        SpawnPoint {
            x,
            y,
            z: f64::from(z),
            heading_deg: place.1.to_degrees(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use opts_traffic::{ConnectParams, LaneGraph, LaneSegment};

    /// A road 100 m north from the origin and one 100 m east from (30, 0), 30 m apart.
    fn graph() -> LaneGraph {
        let north = LaneSegment::straight(bevy::math::DVec2::ZERO, 0.0, 100.0, LaneKind::Road);
        let east = LaneSegment::straight(
            bevy::math::DVec2::new(30.0, 0.0),
            std::f64::consts::FRAC_PI_2,
            100.0,
            LaneKind::Road,
        );
        LaneGraph::build(&[north, east], &ConnectParams::default())
    }

    #[test]
    fn a_bus_beside_a_road_goes_onto_the_lane_that_runs_its_way() {
        // 4 m east of the north road, facing north: the north lane, even though the east lane's
        // start is closer along the other axis.
        let (p, h) = road_place(&graph(), Vec2::new(4.0, 50.0), 0.0).expect("a lane");
        assert!(p.x.abs() < 0.01 && (p.y - 50.0).abs() < 1.5, "{p:?}");
        assert!(h.abs() < 0.01);
    }

    #[test]
    fn a_bus_facing_against_the_lane_keeps_facing_against_it() {
        let (p, h) =
            road_place(&graph(), Vec2::new(2.0, 50.0), std::f32::consts::PI).expect("a lane");
        assert!(p.x.abs() < 0.01);
        assert!((h - std::f64::consts::PI).abs() < 0.01, "{h}");
    }

    #[test]
    fn nothing_within_reach_is_none() {
        assert!(road_place(&graph(), Vec2::new(-500.0, -500.0), 0.0).is_none());
    }
}
