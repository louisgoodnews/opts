//! Session vehicles without a window: switching, despawning, respawning, the place label, lost
//! vehicles and a session that never had a vehicle.

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use opts_bevy::{ArticulatedChain, CameraMode, CameraState, PlayerVehicle, VehicleController};

use super::*;
use crate::app_state::AppState;
use crate::in_game::omsi_world::WorldStats;
use crate::in_game::types::InGameEntity;
use crate::session_vehicles::ops;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, StatesPlugin))
        .init_state::<AppState>()
        .init_resource::<WorldStats>()
        .init_resource::<CameraState>()
        .add_plugins(SessionVehiclesPlugin);
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::InGame);
    for _ in 0..3 {
        app.update();
    }
    app
}

/// A session vehicle that stands at `at` (session frame) and was spawned at entry `entry`.
fn add(app: &mut App, name: &str, entry: &str, at: Vec2) -> (VehicleId, Entity) {
    let mut controller = VehicleController::standard_bus(12_000.0, 5.9);
    controller.sim.chassis.position = at;
    let entity = app
        .world_mut()
        .spawn((controller, InGameEntity, Transform::default()))
        .id();
    let mut reg = app.world_mut().resource_mut::<SessionVehicles>();
    let id = reg.register_alive(
        SpawnParams {
            name: name.to_owned(),
            ..Default::default()
        },
        entity,
        Some((1, entry.to_owned())),
        Some(SpawnPoint {
            x: f64::from(at.x),
            y: f64::from(at.y),
            z: 0.0,
            heading_deg: 0.0,
        }),
        None,
    );
    app.world_mut()
        .entity_mut(entity)
        .insert(SessionVehicle(id));
    (id, entity)
}

fn players(app: &mut App) -> Vec<Entity> {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<PlayerVehicle>>();
    q.iter(app.world()).collect()
}

fn key(app: &mut App, code: KeyCode, logical: bevy::input::keyboard::Key) {
    for state in [
        bevy::input::ButtonState::Pressed,
        bevy::input::ButtonState::Released,
    ] {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: code,
                logical_key: logical.clone(),
                state,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
        run(app, 2);
    }
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

mod despawn;
mod place_label;
mod switching;

mod lifecycle;
