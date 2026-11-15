//! The mini-map widget: rows, redraws, centring, keys and radius.

use super::*;

/// A drive with the HUD, a vehicle with a position, fixture roads, a registry for the settings
/// (saved under a throwaway folder) and a clock that advances 16 ms per frame.
pub(super) fn map_app() -> App {
    use crate::settings_registry::{SettingChanged, SettingsCapabilities, SettingsRegistry};
    let dir = std::env::temp_dir().join(format!("opts-hud-map-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let mut app = app();
    {
        let mut settings = app.world_mut().resource_mut::<GameSettings>();
        settings.db_path_override = Some(dir.join("settings.db"));
    }
    app.insert_resource(SettingsRegistry::with_builtin())
        .init_resource::<SettingsCapabilities>()
        .add_message::<SettingChanged>()
        .insert_resource(FixtureRoads::grid())
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(16),
        ));
    // The vehicle stands at (30 east, 20 north), heading east.
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<opts_bevy::PlayerVehicle>>();
    let player = q.iter(app.world()).next().expect("player");
    app.world_mut().entity_mut(player).insert((
        GlobalTransform::from(
            Transform::from_xyz(30.0, 0.0, -20.0)
                .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        ),
        crate::session_vehicles::SessionVehicle(1),
    ));
    app.world_mut().spawn((
        crate::session_vehicles::SessionVehicle(2),
        GlobalTransform::from(Transform::from_xyz(-60.0, 0.0, 10.0)),
    ));
    build(&mut app, VehicleKind::Bus);
    let profile = app
        .world()
        .resource::<HudProfiles>()
        .get(VehicleKind::Bus)
        .cloned()
        .expect("profile");
    app.insert_resource(build::ActiveProfile {
        kind: VehicleKind::Bus,
        profile,
    });
    app
}

fn minimap_state(app: &App) -> MiniMapState {
    app.world().resource::<MiniMapState>().clone()
}

pub(super) fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}
mod centre;
mod draw;
mod keys;
mod layout;

#[path = "map_widget/tests.rs"]
mod protocol_and_radius;
