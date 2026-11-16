//! The headless ECS synchronisation of OMSI and Bevy transforms.

use super::*;

#[test]
fn test_headless_bevy_sync_systems() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(OptsCoordsPlugin);

    // Spawn an entity with OmsiTransform and default Transform
    let entity = app
        .world_mut()
        .spawn((
            OmsiTransform::from_translation(Vec3::new(1.0, 5.0, 2.0)),
            Transform::default(),
        ))
        .id();

    // First update: OmsiTransform should sync to Transform
    app.update();

    let transform = *app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform exists");
    assert_eq!(transform.translation, Vec3::new(1.0, 2.0, -5.0));

    // Mutate OmsiTransform: change translation
    {
        let mut entity_ref = app.world_mut().entity_mut(entity);
        let mut omsi_tx = entity_ref
            .get_mut::<OmsiTransform>()
            .expect("OmsiTransform exists");
        omsi_tx.translation = Vec3::new(3.0, 8.0, -1.0);
    }

    app.update();

    let updated_tx = *app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform exists");
    assert_eq!(updated_tx.translation, Vec3::new(3.0, -1.0, -8.0));

    // Reverse sync: Mutate Bevy Transform directly
    {
        let mut entity_ref = app.world_mut().entity_mut(entity);
        let mut bevy_tx = entity_ref.get_mut::<Transform>().expect("Transform exists");
        bevy_tx.translation = Vec3::new(5.0, 10.0, -15.0);
    }

    app.update();

    let back_omsi_tx = *app
        .world()
        .entity(entity)
        .get::<OmsiTransform>()
        .expect("OmsiTransform exists");
    assert_eq!(back_omsi_tx.translation, Vec3::new(5.0, 15.0, 10.0));

    // Stability test: Run multiple updates to ensure no runaway feedback loop
    for _ in 0..5 {
        app.update();
    }

    let stable_bevy = *app
        .world()
        .entity(entity)
        .get::<Transform>()
        .expect("Transform exists");
    let stable_omsi = *app
        .world()
        .entity(entity)
        .get::<OmsiTransform>()
        .expect("OmsiTransform exists");
    assert_eq!(stable_bevy.translation, Vec3::new(5.0, 10.0, -15.0));
    assert_eq!(stable_omsi.translation, Vec3::new(5.0, 15.0, 10.0));
}
