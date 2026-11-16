//! Unit tests of `light_cones`.

use std::time::Duration;

use bevy::time::TimeUpdateStrategy;

use super::shape::{
    cone_angles, headlight_cone_shape, lantern_cone_length, lantern_cone_shape,
    spotlight_cone_shape, ConeKind, ConeShape,
};
use super::*;

use crate::light_quality::{ladder_preset, LightMode, LightQualityInput};

/// The input that makes the effective quality exactly `q` (the plugin's resolver rewrites
/// `LightQuality` every frame from the input and the governor).
fn custom(q: LightQuality) -> LightQualityInput {
    LightQualityInput {
        mode: LightMode::Custom,
        custom: q,
    }
}

fn e(i: u32) -> Entity {
    Entity::from_raw_u32(i).expect("entity")
}

fn light(direction: Vec3, omni: bool, cone: bool, inner: f32, outer: f32) -> ScoLight {
    ScoLight {
        position: Vec3::new(1.0, 2.0, 7.0),
        direction,
        color: [255.0, 128.0, 0.0],
        size_m: 1.0,
        variable: "NightlightA".into(),
        factor: 1.0,
        omni,
        cone_inner_deg: inner,
        cone_outer_deg: outer,
        time_const_s: 0.0,
        volumetric_cone: cone,
        bitmap: String::new(),
        after_mesh: None,
        lod_threshold: None,
    }
}

// ---- mapping ------------------------------------------------------------------------------

mod mapping;

mod prepass;
use prepass::main_camera;

// ---- selection ----------------------------------------------------------------------------

mod selection;

// ---- the system in a headless app ---------------------------------------------------------

fn app(level: u8) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            50,
        )))
        .insert_resource(custom(ladder_preset(level)))
        .add_plugins(LightConePlugin);
    app.world_mut().spawn((
        Camera::default(),
        crate::MainCamera,
        Transform::from_xyz(0.0, 1.7, 0.0),
    ));
    app
}

fn shape() -> ConeShape {
    ConeShape {
        apex: Vec3::ZERO,
        dir: Vec3::NEG_Y,
        length: 6.0,
        half_angle: 30f32.to_radians(),
        inner_frac: 0.3,
        color: [1.0; 3],
        ground_h: Some(5.0),
    }
}

/// A cone `z` metres in front of the camera.
fn spawn_cone(app: &mut App, kind: ConeKind, level: ConeLevel, z: f32) -> Entity {
    app.world_mut()
        .spawn((
            Transform::from_xyz(0.0, 5.0, -z),
            Visibility::Hidden,
            MeshTag::new(0),
            LightCone {
                kind,
                shape: shape(),
                level,
            },
        ))
        .id()
}

fn visible(app: &App, cones: &[Entity]) -> usize {
    cones
        .iter()
        .filter(|c| *app.world().get::<Visibility>(**c).unwrap() == Visibility::Inherited)
        .count()
}

fn run(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn spawn_mixed(app: &mut App) -> (Vec<Entity>, Vec<Entity>) {
    let lanterns = (1..=3)
        .map(|i| {
            spawn_cone(
                app,
                ConeKind::Lantern,
                ConeLevel::Fixed(1.0),
                10.0 * i as f32,
            )
        })
        .collect();
    let heads = (1..=3)
        .map(|i| {
            spawn_cone(
                app,
                ConeKind::Headlight,
                ConeLevel::Fixed(1.0),
                12.0 * i as f32,
            )
        })
        .collect();
    (lanterns, heads)
}

mod ladder;

mod lighting;

mod cost;

// ---- depth prepass ------------------------------------------------------------------------

// ---- cost ---------------------------------------------------------------------------------
