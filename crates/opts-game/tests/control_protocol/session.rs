//! Session commands: vehicles, spawn, switch, despawn and respawn.

use super::*;

fn session_app() -> App {
    let mut app = base_app(Vec::new());
    app.add_plugins(SessionVehiclesPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

/// A session vehicle of the registry with a simulation, standing at `x`.
fn add_vehicle(app: &mut App, name: &str, entry: &str, x: f32) -> u64 {
    let mut controller = VehicleController::standard_bus(12_000.0, 5.9);
    controller.sim.chassis.position = Vec2::new(x, 0.0);
    let entity = app
        .world_mut()
        .spawn((controller, InGameEntity, Transform::default()))
        .id();
    let id = app
        .world_mut()
        .resource_mut::<SessionVehicles>()
        .register_alive(
            SpawnParams {
                name: name.to_owned(),
                ..Default::default()
            },
            entity,
            Some((1, entry.to_owned())),
            Some(SpawnPoint {
                x: f64::from(x),
                y: 0.0,
                z: 0.0,
                heading_deg: 0.0,
            }),
            None,
        );
    app.world_mut()
        .entity_mut(entity)
        .insert(SessionVehicle(id));
    u64::from(id)
}

mod change_map;
mod spawn;
mod switch;
