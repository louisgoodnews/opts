//! A parked car body follows the daylight.

use super::*;

#[test]
fn a_parked_car_body_follows_the_daylight_and_other_variables_stay_out() {
    let mut app = app();
    // Noon on a summer day: the sun is high, `Envir_Brightness` is 1.
    app.insert_resource(opts_bevy::SceneryClock::at(2026, 6, 21, 12.0 * 3600.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    // A vehicle model placed as scenery: no scripts, no AI table.
    let owner = app.world_mut().spawn(Transform::default()).id();
    let mut part = |var: &str| {
        app.world_mut()
            .spawn((
                AlphaScaleBinding::new(owner, var),
                MeshMaterial3d(material.clone()),
                Visibility::Hidden,
            ))
            .id()
    };
    let body = part("Envir_Brightness");
    let dirt = part("Dirt_Norm");
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(body).unwrap(),
        Visibility::Inherited,
        "the body shows by day"
    );
    assert_eq!(
        *app.world().get::<Visibility>(dirt).unwrap(),
        Visibility::Hidden
    );
}
