//! Window glass follows its alpha scale variable.

use super::*;

/// MAN SD83: window glass (`Fenster.tga`, alpha 2, `[alphascale] Envir_Brightness`) is hidden at
/// night-time brightness 0, tinted in proportion to the brightness by day, on a private copy.
#[test]
fn glass_follows_its_alpha_scale_variable() {
    let mut app = app();
    let owner = app.world_mut().spawn(AiVariables::default()).id();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgba(0.0, 0.0, 0.0, 0.5),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
    let glass = app
        .world_mut()
        .spawn((
            AlphaScaleBinding::new(owner, "Envir_Brightness"),
            MeshMaterial3d(handle.clone()),
            Visibility::Hidden,
        ))
        .id();
    let alpha = |app: &App| {
        let h = &app
            .world()
            .get::<MeshMaterial3d<StandardMaterial>>(glass)
            .unwrap()
            .0;
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .get(h)
            .unwrap()
            .base_color
            .alpha()
    };
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(glass).unwrap(),
        Visibility::Hidden,
        "variable 0: no glass"
    );
    app.world_mut()
        .get_mut::<AiVariables>(owner)
        .unwrap()
        .set("envir_brightness", 1.0);
    app.update();
    assert_eq!(
        *app.world().get::<Visibility>(glass).unwrap(),
        Visibility::Inherited
    );
    assert!((alpha(&app) - 0.5).abs() < 1e-6);
    app.world_mut()
        .get_mut::<AiVariables>(owner)
        .unwrap()
        .set("envir_brightness", 0.4);
    app.update();
    assert!((alpha(&app) - 0.2).abs() < 1e-6, "base alpha 0.5 x 0.4");
    // The shared material is untouched.
    let shared = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .get(&handle)
        .unwrap()
        .base_color
        .alpha();
    assert!((shared - 0.5).abs() < 1e-6);
}
