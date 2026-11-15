//! The interior view dims the camera's ambient light, and the exterior restores it.

use super::*;

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
