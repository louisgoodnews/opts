//! Unit tests of `light_rooms`.

use std::time::Duration;

use bevy::camera::visibility::{ViewVisibility, VisibilityPlugin};
use bevy::time::TimeUpdateStrategy;

use super::*;

// ---- the state machine --------------------------------------------------------------------

#[test]
fn the_cabin_layer_has_hysteresis_and_an_interior_view_always_wants_it() {
    // Outside, entering at 12 m, leaving at 15 m.
    assert!(!cabin_layer_wanted(false, false, Some(13.0)));
    assert!(cabin_layer_wanted(false, false, Some(12.0)));
    assert!(cabin_layer_wanted(false, false, Some(3.0)));
    assert!(
        cabin_layer_wanted(true, false, Some(14.0)),
        "stays between 12 and 15"
    );
    assert!(cabin_layer_wanted(true, false, Some(15.0)));
    assert!(!cabin_layer_wanted(true, false, Some(15.1)));
    assert!(
        !cabin_layer_wanted(false, false, Some(14.0)),
        "does not join between 12 and 15"
    );
    // An interior view wants it at any distance (a camera glued into the cockpit).
    assert!(cabin_layer_wanted(false, true, Some(40.0)));
    // No player vehicle: never.
    assert!(!cabin_layer_wanted(true, true, None));
    assert!(!cabin_layer_wanted(false, false, None));
}

#[test]
fn a_walk_away_and_back_switches_once_each_way() {
    let mut on = false;
    let mut switches = 0;
    // 30 m away, in to 5 m, out to 30 m, in steps of 0.5 m with a wobble of +-0.4 m.
    let path = (0..120)
        .map(|i| 30.0 - i as f32 * 0.5)
        .chain((0..120).map(|i| 5.0 + i as f32 * 0.5));
    for (i, d) in path.enumerate() {
        let wobble = if i % 2 == 0 { 0.4 } else { -0.4 };
        let next = cabin_layer_wanted(on, false, Some(d + wobble));
        if next != on {
            switches += 1;
        }
        on = next;
    }
    assert_eq!(switches, 2);
}

#[test]
fn the_cabin_camera_layers_are_world_plus_cabin() {
    let outside = camera_layers(false);
    let inside = camera_layers(true);
    let world = RenderLayers::layer(WORLD_LAYER);
    let cabin = RenderLayers::layer(CABIN_LAYER);
    // An entity without the component is on layer 0 and must stay visible either way.
    assert!(outside.intersects(&RenderLayers::default()));
    assert!(inside.intersects(&RenderLayers::default()));
    assert!(!outside.intersects(&cabin));
    assert!(inside.intersects(&cabin));
    assert!(inside.intersects(&world) && outside.intersects(&world));
}

// ---- footprint ----------------------------------------------------------------------------

fn footprint() -> VehicleFootprint {
    VehicleFootprint {
        min: Vec3::new(-1.3, 0.0, -6.0),
        max: Vec3::new(1.3, 3.2, 6.0),
    }
}

#[test]
fn a_lamp_over_the_roof_is_over_the_vehicle() {
    let fp = footprint();
    let at = |x, y, z| {
        over_vehicle(
            Vec3::new(x, y, z),
            &GlobalTransform::default(),
            &fp,
            FOOTPRINT_MARGIN_M,
        )
    };
    assert!(at(0.0, 7.0, 0.0), "straight above");
    assert!(at(2.0, 7.0, 6.5), "within the margin");
    assert!(!at(2.6, 7.0, 0.0), "beside the vehicle");
    assert!(!at(0.0, 7.0, 8.0), "ahead of it");
    assert!(!at(0.0, 2.0, 0.0), "below the roof");
    // The vehicle turned 90 degrees: its length now lies along X.
    let turned = GlobalTransform::from(Transform::from_rotation(Quat::from_rotation_y(
        std::f32::consts::FRAC_PI_2,
    )));
    assert!(over_vehicle(Vec3::new(5.0, 7.0, 0.0), &turned, &fp, 1.0));
    assert!(!over_vehicle(Vec3::new(0.0, 7.0, 5.0), &turned, &fp, 1.0));
}

#[test]
fn the_distance_to_the_footprint_is_zero_inside() {
    let fp = footprint();
    assert_eq!(fp.distance(Vec3::new(0.0, 1.0, 0.0)), 0.0);
    assert!((fp.distance(Vec3::new(4.3, 1.0, 0.0)) - 3.0).abs() < 1e-5);
    assert!((fp.distance(Vec3::new(1.3, 3.2 + 4.0, 6.0 + 3.0)) - 5.0).abs() < 1e-5);
}

// ---- the system ---------------------------------------------------------------------------

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

#[test]
fn the_main_camera_joins_and_leaves_the_cabin_layer_by_distance() {
    let mut app = app();
    // 30 m from the footprint's side.
    let cam = camera(&mut app, Vec3::new(31.3, 1.7, 0.0));
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(false));
    assert!(!app.world().resource::<CabinView>().cabin_layer);
    move_camera(&mut app, cam, Vec3::new(1.3 + 13.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(false), "13 m: not yet");
    move_camera(&mut app, cam, Vec3::new(1.3 + 11.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(true), "11 m: in");
    move_camera(&mut app, cam, Vec3::new(1.3 + 14.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(true), "14 m: still in");
    assert!(app.world().resource::<CabinView>().cabin_layer);
    move_camera(&mut app, cam, Vec3::new(1.3 + 16.0, 1.7, 0.0));
    assert_eq!(layers(&app, cam), camera_layers(false), "16 m: out");
}

#[test]
fn an_interior_view_includes_the_cabin_layer_and_is_published() {
    let mut app = app();
    let cam = camera(&mut app, Vec3::new(60.0, 1.7, 0.0));
    app.insert_resource(crate::camera::CameraState {
        mode: crate::camera::CameraMode::Cockpit,
        ..default()
    });
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(true));
    let view = *app.world().resource::<CabinView>();
    assert!(view.interior_view && view.cabin_layer);
    // Back to the external view: out again.
    app.insert_resource(crate::camera::CameraState {
        mode: crate::camera::CameraMode::External,
        ..default()
    });
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(false));
    assert!(!app.world().resource::<CabinView>().interior_view);
}

#[test]
fn without_a_player_vehicle_the_camera_stays_on_the_world_layer() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::transform::TransformPlugin))
        .add_plugins(LightRoomsPlugin);
    let cam = camera(&mut app, Vec3::ZERO);
    app.update();
    app.update();
    assert_eq!(layers(&app, cam), camera_layers(false));
}

// ---- the claim the convention rests on ----------------------------------------------------

/// A light on the cabin layer is in no view unless the camera includes that layer; entities
/// without `RenderLayers` (layer 0) stay visible to a camera on `{0, 1}`. This is the path
/// both clustering modes depend on: the CPU assignment checks the layers again
/// (`bevy_light::cluster::assign`, "view_layers.intersects"), and the GPU clustering takes only
/// lights that were extracted, which needs a `ViewVisibility` (`bevy_pbr::render::light`,
/// "if !view_visibility.get()").
#[test]
fn a_cabin_light_is_view_visible_only_to_a_camera_with_the_cabin_layer() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::transform::TransformPlugin,
        VisibilityPlugin,
    ))
    .init_resource::<Assets<Mesh>>()
    .init_resource::<Assets<bevy::mesh::skinning::SkinnedMeshInverseBindposes>>();
    let cam = app
        .world_mut()
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();
    let cabin_light = app
        .world_mut()
        .spawn((
            PointLight::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
            RenderLayers::layer(CABIN_LAYER),
        ))
        .id();
    let world_light = app
        .world_mut()
        .spawn((PointLight::default(), Transform::from_xyz(0.5, 0.0, 0.0)))
        .id();
    let seen = |app: &App, e: Entity| app.world().get::<ViewVisibility>(e).unwrap().get();

    // No component on the camera: layer 0 only.
    app.update();
    app.update();
    assert!(seen(&app, world_light), "a layer-0 light is seen");
    assert!(
        !seen(&app, cabin_light),
        "a camera on layer 0 excludes cabin lights"
    );

    app.world_mut().entity_mut(cam).insert(camera_layers(false));
    app.update();
    app.update();
    assert!(seen(&app, world_light));
    assert!(!seen(&app, cabin_light));

    app.world_mut().entity_mut(cam).insert(camera_layers(true));
    app.update();
    app.update();
    assert!(seen(&app, world_light), "{{0, 1}} keeps layer-0 entities");
    assert!(seen(&app, cabin_light), "{{0, 1}} includes cabin lights");

    app.world_mut().entity_mut(cam).insert(camera_layers(false));
    app.update();
    app.update();
    assert!(
        !seen(&app, cabin_light),
        "leaving the layer drops them again"
    );
}

#[test]
fn an_interior_view_dims_the_ambient_light_of_the_camera_and_the_exterior_restores_it() {
    let mut app = App::new();
    app.insert_resource(CabinView::default())
        .insert_resource(GlobalAmbientLight {
            brightness: 200.0,
            ..default()
        })
        .add_systems(Update, interior_ambient_system);
    let cam = app
        .world_mut()
        .spawn((crate::MainCamera, Camera3d::default()))
        .id();
    app.update();
    assert!(app.world().get::<AmbientLight>(cam).is_none());
    app.world_mut().resource_mut::<CabinView>().interior_view = true;
    app.update();
    let dimmed = app
        .world()
        .get::<AmbientLight>(cam)
        .expect("own ambient")
        .brightness;
    assert!((dimmed - 200.0 * INTERIOR_AMBIENT_SCALE).abs() < 1e-3);
    app.world_mut().resource_mut::<CabinView>().interior_view = false;
    app.update();
    assert!(app.world().get::<AmbientLight>(cam).is_none());
    assert!(app.world().get::<InteriorAmbient>(cam).is_none());
}
