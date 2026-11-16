//! The lighting-quality section of the graphics page: the automatic governor shows its values, and
//! the custom mode unlocks the controls.

use super::*;
use opts_bevy::light_quality::{ladder_preset, GovernorState, LightMode, LightQuality};

fn quality(app: &App) -> LightQuality {
    *app.world().resource::<LightQuality>()
}

#[test]
fn automatic_shows_the_governors_values_disabled_and_custom_unlocks_them() {
    let (mut app, _, _) = page_app("lightquality", "graphics");
    assert!(by_id(
        &mut app,
        "settings/graphics/graphics.light_quality_mode/option-custom"
    )
    .is_some());
    let cap_up = "settings/graphics/graphics.light_cone_cap/up";
    let up = by_id(&mut app, cap_up).expect("cone cap stepper");
    assert!(
        app.world().get::<Disabled>(up).is_some(),
        "automatic: the custom controls are disabled"
    );
    let bloom = by_id(&mut app, "settings/graphics/graphics.bloom/option-natural").expect("bloom");
    assert!(app.world().get::<Disabled>(bloom).is_some());
    assert!(
        by_id(&mut app, "settings/graphics/graphics.bloom/reset").is_none(),
        "nothing to reset while the governor decides"
    );

    // The governor moves to level 3: the page shows its values (cone cap 0) without
    // touching the stored custom values.
    app.world_mut().resource_mut::<GovernorState>().level = 3;
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(quality(&app), ladder_preset(3));
    let shown = app
        .world_mut()
        .query::<(&super::super::rows::SettingValueText, &Text)>()
        .iter(app.world())
        .find(|(k, _)| k.0 == "graphics.light_cone_cap")
        .map(|(_, t)| t.0.clone());
    assert_eq!(shown.as_deref(), Some("0"));
    assert_eq!(
        text(&app, "graphics.light_cone_cap"),
        "64",
        "stored value untouched"
    );

    // Custom: the level reached becomes the starting values and the controls work.
    let custom = by_id(
        &mut app,
        "settings/graphics/graphics.light_quality_mode/option-custom",
    )
    .expect("custom option");
    click(&mut app, custom);
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(text(&app, "graphics.light_quality_mode"), "custom");
    assert_eq!(text(&app, "graphics.light_cone_cap"), "0");
    assert_eq!(text(&app, "graphics.light_headlight_cones"), "false");
    let up = by_id(&mut app, cap_up).expect("cone cap stepper");
    assert!(app.world().get::<Disabled>(up).is_none(), "custom: enabled");
    click(&mut app, up);
    app.update();
    app.update();
    assert_eq!(text(&app, "graphics.light_cone_cap"), "8");
    assert_eq!(quality(&app).cone_cap, 8);
    assert_eq!(quality(&app).mode, LightMode::Custom);
    // The governor's target frame rate is the other way round: greyed out in custom.
    let fps_up = by_id(
        &mut app,
        "settings/graphics/graphics.light_governor_target_fps/up",
    )
    .expect("target fps stepper");
    assert!(
        app.world().get::<Disabled>(fps_up).is_some(),
        "custom: the governor target is disabled"
    );
}

#[test]
fn the_governor_target_is_enabled_in_automatic_mode() {
    let (mut app, _, _) = page_app("lightgovfps", "graphics");
    let up = by_id(
        &mut app,
        "settings/graphics/graphics.light_governor_target_fps/up",
    )
    .expect("target fps stepper");
    assert!(app.world().get::<Disabled>(up).is_none());
    click(&mut app, up);
    app.update();
    app.update();
    assert_eq!(text(&app, "graphics.light_governor_target_fps"), "70");
    assert_eq!(
        app.world()
            .resource::<opts_bevy::light_quality::GovernorTargetFps>()
            .0,
        70
    );
}

#[test]
fn the_control_protocol_sets_the_mode() {
    let (mut app, _, _) = page_app("lightproto", "graphics");
    let out = crate::settings_registry::protocol::set(
        app.world_mut(),
        &serde_json::json!({ "key": "graphics.light_quality_mode", "value": "custom" }),
    );
    assert!(
        outcome_text(out).contains("custom"),
        "settings.set must accept the mode"
    );
    app.update();
    app.update();
    assert_eq!(quality(&app).mode, LightMode::Custom);
    assert_eq!(text(&app, "graphics.light_quality_mode"), "custom");
}
