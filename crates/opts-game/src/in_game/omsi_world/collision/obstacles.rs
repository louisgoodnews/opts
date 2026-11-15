//! The AI vehicles' boxes (measured once their meshes have bounds) and the player's footprints, handed
//! to the AI traffic as obstacles.

use bevy::camera::primitives::Aabb;
use bevy::prelude::*;
use opts_bevy::{ArticulatedChain, ArticulatedSegment, PlayerVehicle, VehicleController};

use super::super::rail::VehicleComposition;
use super::super::traffic::TrafficVehicle;
use super::super::RailPlayer;
use super::body::{BodyBox, PlayerBox, VehicleCollider};
use super::boxes::player_boxes;

/// Largest half extents `(right, up, back)` the box of an AI vehicle may have: 3.6 m wide, 5 m
/// high, 20 m long.
pub(super) const MAX_VEHICLE_HALF: Vec3 = Vec3::new(1.8, 2.5, 10.0);
/// AI vehicles whose box has not been measured yet.
pub(super) type Unmeasured = (With<TrafficVehicle>, Without<VehicleCollider>);

/// What the player's collision pass reads and moves.
pub(super) type PlayerCollide = (
    &'static mut VehicleController,
    &'static PlayerBox,
    Option<&'static ArticulatedChain>,
    Option<&'static VehicleComposition>,
);

/// Road vehicles of the player (rail vehicles are ghosts).
pub(super) type GroundPlayer = (
    Or<(
        With<crate::session_vehicles::SessionVehicle>,
        With<PlayerVehicle>,
    )>,
    Without<RailPlayer>,
);

/// Measures the box of AI vehicles from their meshes once these have bounds.
pub fn attach_traffic_colliders(
    mut commands: Commands,
    vehicles: Query<(Entity, &GlobalTransform), Unmeasured>,
    children: Query<&Children>,
    bounds: Query<(
        &Aabb,
        &GlobalTransform,
        Has<opts_bevy::light_cones::LightCone>,
    )>,
) {
    for (e, root) in &vehicles {
        let inverse = root.affine().inverse();
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        let mut count = 0;
        for d in children.iter_descendants(e) {
            let Ok((aabb, g, is_cone)) = bounds.get(d) else {
                continue;
            };
            // The headlight cones reach 8 m out of a car; they are light, not body.
            if is_cone {
                continue;
            }
            let (c, h) = (Vec3::from(aabb.center), Vec3::from(aabb.half_extents));
            // Shadow volumes and helper geometry are far larger than a vehicle.
            if h.max_element() > 9.0 {
                continue;
            }
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    for sz in [-1.0, 1.0] {
                        let p = g.transform_point(c + h * Vec3::new(sx, sy, sz));
                        let local = inverse.transform_point3(p);
                        lo = lo.min(local);
                        hi = hi.max(local);
                    }
                }
            }
            count += 1;
        }
        if count >= 3 && hi.x > lo.x {
            // A road vehicle is at most so wide, high and long; more is helper geometry that got
            // through (a car measured 29 by 34 m once and pushed the player about).
            let half = ((hi - lo) * 0.5).min(MAX_VEHICLE_HALF);
            commands.entity(e).insert(VehicleCollider {
                centre: (lo + hi) * 0.5,
                half,
            });
        }
    }
}
/// Hands the AI traffic the footprints of the player's road vehicles (head and following parts),
/// so the agents stop behind them or change lanes round them (`opts_traffic::obstacles`) instead
/// of driving into them; the parked vehicles near the camera count too.
pub fn publish_traffic_obstacles(
    mut state: ResMut<super::super::traffic::TrafficState>,
    players: Query<PlayerCollide, GroundPlayer>,
    segments: Query<&ArticulatedSegment>,
    camera: Query<&GlobalTransform, With<opts_bevy::MainCamera>>,
) {
    let mut obstacles = vec![];
    for (controller, head, chain, composition) in &players {
        let part_boxes: Vec<BodyBox> = composition
            .map(|c| c.followers.iter().filter_map(|p| p.body).collect())
            .unwrap_or_default();
        let followers: Vec<(BodyBox, Vec2, f32)> = chain
            .map(|c| {
                c.segments
                    .iter()
                    .zip(&part_boxes)
                    .filter_map(|(e, b)| {
                        let (p, h) = segments.get(*e).ok()?.simulated_pose()?;
                        Some((*b, p, h))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let ch = &controller.sim.chassis;
        let v = ch.world_velocity();
        let boxes = player_boxes(
            ch.position,
            ch.heading_rad,
            ch.elevation_z,
            head,
            &followers,
        );
        obstacles.extend(boxes.into_iter().map(|b| opts_traffic::Obstacle {
            centre: bevy::math::DVec2::new(f64::from(b.centre.x), f64::from(b.centre.y)),
            heading: f64::from(b.heading),
            length: f64::from(b.half.y) * 2.0,
            width: f64::from(b.half.x) * 2.0,
            velocity: bevy::math::DVec2::new(f64::from(v.x), f64::from(v.y)),
        }));
    }
    state.set_obstacles_with_parked(obstacles, camera.single().ok());
}
