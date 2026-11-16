//! Lighting quality: the custom mode applies live and the mode changes copy or restart the level.

use super::*;

#[test]
fn custom_mode_applies_the_controls_live() {
    let mut app = app();
    assert_eq!(quality(&app), ladder_preset(0));
    // Automatic: the controls do not change what is applied.
    set(&mut app, "graphics.bloom", "natural");
    app.update();
    assert_eq!(quality(&app).bloom, BloomLevel::Off);
    // Custom: they are authoritative.
    set(&mut app, "graphics.light_quality_mode", "custom");
    app.update();
    app.update();
    let q = quality(&app);
    assert_eq!(q.mode, LightMode::Custom);
    assert_eq!(
        q.bloom,
        BloomLevel::Off,
        "the switch copied the automatic level over the old custom values"
    );
    set(&mut app, "graphics.bloom", "natural");
    set(&mut app, "graphics.light_shafts", "ultra");
    set(&mut app, "graphics.light_cone_cap", "96");
    set(&mut app, "graphics.mirror_rate", "4");
    app.update();
    app.update();
    let q = quality(&app);
    assert_eq!(
        (q.shafts, q.cone_cap, q.mirror_every),
        (Shafts::Ultra, 96, 4)
    );
    assert_eq!(
        app.world().resource::<LightQualityInput>().mode,
        LightMode::Custom
    );
}

#[test]
fn automatic_to_custom_copies_the_level_reached() {
    let mut app = app();
    app.world_mut().resource_mut::<GovernorState>().level = 2;
    app.update();
    assert_eq!(quality(&app), ladder_preset(2));
    set(&mut app, "graphics.light_quality_mode", "custom");
    for _ in 0..3 {
        app.update();
    }
    let q = quality(&app);
    assert_eq!((q.mode, q.level), (LightMode::Custom, 2));
    assert!(q.same_values(&ladder_preset(2)), "{q:?}");
    let settings = app.world().resource::<GameSettings>();
    assert!(lighting::custom_of(settings).same_values(&ladder_preset(2)));
    assert_eq!(
        settings.raw("graphics.light_cone_cap").as_deref(),
        Some("16")
    );
    assert_eq!(
        settings.raw("graphics.light_lantern_cones").as_deref(),
        Some("false")
    );
}

#[test]
fn custom_to_automatic_restarts_at_level_zero() {
    let mut app = app();
    app.world_mut().resource_mut::<GovernorState>().level = 4;
    app.update();
    set(&mut app, "graphics.light_quality_mode", "custom");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(quality(&app).level, 4);
    set(&mut app, "graphics.light_quality_mode", "automatic");
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(quality(&app), ladder_preset(0));
    assert_eq!(app.world().resource::<GovernorState>().level, 0);
}
