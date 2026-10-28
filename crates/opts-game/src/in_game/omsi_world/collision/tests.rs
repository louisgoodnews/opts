//! Unit tests for `super` (`collision.rs`).

use super::*;
use bevy::camera::primitives::Aabb;

/// A car whose meshes include helper geometry far out of the body (8 m headlight cones used
/// to) still gets a box of vehicle size: the AI car that measured 29 by 34 m pushed the
/// player's standing bus 40 m across the map.
#[test]
fn a_vehicle_box_is_never_larger_than_a_vehicle() {
    let mut app = App::new();
    app.add_systems(Update, attach_traffic_colliders);
    let root = app
        .world_mut()
        .spawn((
            super::super::traffic::TrafficVehicle::at(bevy::math::DVec2::ZERO, 0.0, 0.0),
            GlobalTransform::default(),
        ))
        .id();
    for (centre, half) in [
        (Vec3::ZERO, Vec3::new(0.9, 0.7, 2.2)),
        (Vec3::new(0.0, 0.5, 0.5), Vec3::new(0.8, 0.5, 1.5)),
        (Vec3::new(0.0, 0.3, -0.5), Vec3::new(0.7, 0.3, 1.0)),
        // Something that reaches 8 m out sideways and 12 m forwards.
        (Vec3::new(8.0, 0.0, 12.0), Vec3::new(8.5, 8.5, 8.5)),
    ] {
        let child = app
            .world_mut()
            .spawn((
                Aabb {
                    center: centre.into(),
                    half_extents: half.into(),
                },
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().entity_mut(root).add_child(child);
    }
    app.update();
    let c = app
        .world()
        .get::<VehicleCollider>(root)
        .expect("the box was measured");
    assert!(c.half.x <= MAX_VEHICLE_HALF.x, "{:?}", c.half);
    assert!(c.half.y <= MAX_VEHICLE_HALF.y, "{:?}", c.half);
    assert!(c.half.z <= MAX_VEHICLE_HALF.z, "{:?}", c.half);
}
