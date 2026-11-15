//! The claim the layer convention rests on: a cabin light is seen only through the cabin layer.

use super::*;

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
