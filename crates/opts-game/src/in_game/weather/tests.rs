//! Unit tests for `super` (`mod.rs`).

use super::*;
use crate::in_game::types::InGameCamera;
use bevy::pbr::DistanceFog;
use opts_bevy::weather::PrecipitationQuality;
use opts_bevy::{CalendarDate, EnvironmentChange, EnvironmentPlugin, SessionEnvironment};
use opts_core::formats::weather::PrecipKind;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), EnvironmentPlugin))
        .init_asset::<Image>()
        .init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<opts_bevy::ground_material::GroundMaterial>()
        .init_resource::<opts_bevy::reflections::Wetness>()
        .init_resource::<opts_bevy::weather::GroundFogView>()
        .init_resource::<clouds::CloudState>()
        .init_resource::<precip::PrecipState>()
        .add_systems(
            Update,
            (
                fog::measure_fog_view,
                fog::update_fog.after(fog::measure_fog_view),
                clouds::update_clouds,
                precip::update_precipitation,
                (surfaces::drive_wetness, surfaces::update_surfaces).chain(),
            )
                .after(EnvironmentSet),
        );
    app.world_mut()
        .spawn((InGameCamera, GlobalTransform::default()));
    app.world_mut().write_message(EnvironmentChange::reset(
        CalendarDate::new(2026, 6, 29),
        43_200.0,
        "Clear Summer Day",
    ));
    app.update();
    app
}

fn set_weather(app: &mut App, id: &str) {
    app.world_mut().write_message(EnvironmentChange {
        weather: Some(id.into()),
        ..default()
    });
    app.update();
    app.update();
}

fn camera_has_fog(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<Has<DistanceFog>, With<InGameCamera>>();
    q.single(app.world()).unwrap()
}

fn precip_visible(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<&Visibility, With<precip::PrecipLayer>>();
    q.iter(app.world()).any(|v| *v == Visibility::Visible)
}

fn fog_density_of(app: &mut App) -> f32 {
    let mut q = app
        .world_mut()
        .query_filtered::<&DistanceFog, With<InGameCamera>>();
    match q.single(app.world()).unwrap().falloff {
        bevy::pbr::FogFalloff::ExponentialSquared { density } => density,
        ref other => panic!("{other:?}"),
    }
}

fn place_camera(app: &mut App, y: f32, pitch_deg: f32) {
    let mut q = app
        .world_mut()
        .query_filtered::<&mut GlobalTransform, With<InGameCamera>>();
    let mut tf = q.single_mut(app.world_mut()).unwrap();
    *tf = GlobalTransform::from(
        Transform::from_xyz(0.0, y, 0.0)
            .with_rotation(Quat::from_rotation_x(pitch_deg.to_radians())),
    );
    app.update();
}

mod effects;
mod scenery;
