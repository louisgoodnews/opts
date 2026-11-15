//! Unit tests of `light_rooms`: the shared fixtures, and the split by concern in `tests/`.

use std::time::Duration;

use bevy::camera::visibility::{ViewVisibility, VisibilityPlugin};
use bevy::time::TimeUpdateStrategy;

use super::*;

mod ambient;
mod claim;
mod footprint;
mod state;
mod system;

fn footprint() -> VehicleFootprint {
    VehicleFootprint {
        min: Vec3::new(-1.3, 0.0, -6.0),
        max: Vec3::new(1.3, 3.2, 6.0),
    }
}

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )))
        .add_plugins(LightRoomsPlugin);
    app.world_mut().spawn((
        PlayerVehicle,
        footprint(),
        Transform::default(),
        Visibility::Inherited,
    ));
    app
}

fn camera(app: &mut App, at: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Camera::default(),
            crate::MainCamera,
            Transform::from_translation(at),
        ))
        .id()
}

fn move_camera(app: &mut App, cam: Entity, at: Vec3) {
    app.world_mut()
        .get_mut::<Transform>(cam)
        .unwrap()
        .translation = at;
    app.update();
    app.update();
}

fn layers(app: &App, cam: Entity) -> RenderLayers {
    app.world()
        .get::<RenderLayers>(cam)
        .expect("layers")
        .clone()
}
