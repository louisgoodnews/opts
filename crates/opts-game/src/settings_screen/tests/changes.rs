//! Choosing, resetting, searching and risky changes.

use super::*;

mod risky_tests;

#[test]
fn choosing_an_option_changes_and_saves_the_setting() {
    let (mut app, _, db) = page_app("choose", "graphics");
    let b = by_id(&mut app, "settings/graphics/texture_filtering/option-8").expect("option");
    click(&mut app, b);
    app.update();
    app.update();
    assert_eq!(text(&app, "texture_filtering"), "8");
    assert_eq!(
        app.world().resource::<GameSettings>().texture_filtering,
        opts_bevy::TextureFiltering::X8
    );
    assert_eq!(stored(&db, "texture_filtering").as_deref(), Some("8"));
}

#[test]
fn reset_restores_the_default_and_is_disabled_at_the_default() {
    let (mut app, _, db) = page_app("reset", "graphics");
    let reset = by_id(&mut app, "settings/graphics/look/reset").expect("reset");
    assert!(
        app.world().get::<Disabled>(reset).is_some(),
        "at default: nothing to reset"
    );
    let b = by_id(&mut app, "settings/graphics/look/option-natural").expect("option");
    click(&mut app, b);
    app.update();
    app.update();
    assert_eq!(text(&app, "look"), "natural");
    let reset = by_id(&mut app, "settings/graphics/look/reset").expect("reset");
    assert!(app.world().get::<Disabled>(reset).is_none());
    click(&mut app, reset);
    app.update();
    app.update();
    assert_eq!(text(&app, "look"), "vivid");
    assert_eq!(stored(&db, "look").as_deref(), Some("vivid"));
}

#[test]
fn search_lists_matches_across_categories() {
    let (mut app, _, _) = page_app("search", "graphics");
    app.world_mut().resource_mut::<SettingsPageState>().query = "language".into();
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    app.update();
    app.update();
    assert!(by_id(&mut app, "settings/interface/interface.language/option-de").is_some());
    assert!(by_id(&mut app, "settings/graphics/look/option-vivid").is_none());
    app.world_mut().resource_mut::<SettingsPageState>().query = "zzzzqq".into();
    app.world_mut().resource_mut::<SettingsPageState>().dirty = true;
    app.update();
    app.update();
    assert!(by_id(&mut app, "settings/interface/interface.language/option-de").is_none());
}

#[test]
fn planned_rows_are_visible_disabled_and_marked() {
    let (mut app, _, _) = page_app("notyet", "gameplay");
    let b = by_id(&mut app, "settings/gameplay/gameplay.time_scale/down")
        .expect("stepper stays visible");
    assert!(app.world().get::<Disabled>(b).is_some());
    let labels = app
        .world_mut()
        .query_filtered::<(), With<opts_gui::not_yet::NotYetLabel>>()
        .iter(app.world())
        .count();
    assert_eq!(labels, 1, "one red label per planned gameplay row");
}

#[test]
fn a_preset_chip_sets_the_ui_scale_and_the_matching_chip_is_marked() {
    let (mut app, _, db) = page_app("scale-presets", "interface");
    for p in ["100", "125", "150", "200"] {
        assert!(
            by_id(&mut app, &format!("settings/interface/ui_scale/preset-{p}")).is_some(),
            "chip {p}"
        );
    }
    // A marked chip has another background role than the plain one (200 stays unmarked here).
    let marked = |app: &mut App, p: &str| {
        let role = |app: &mut App, p: &str| {
            by_id(app, &format!("settings/interface/ui_scale/preset-{p}"))
                .and_then(|e| app.world().get::<BackgroundRole>(e).map(|r| r.0))
        };
        role(app, p) != role(app, "200")
    };
    assert!(
        marked(&mut app, "100"),
        "100 % is the default and is marked"
    );
    assert!(!marked(&mut app, "150"));
    let b = by_id(&mut app, "settings/interface/ui_scale/preset-150").expect("chip");
    click(&mut app, b);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<GameSettings>().ui_scale, 150);
    assert_eq!(stored(&db, "ui_scale").as_deref(), Some("150"));
    assert!(
        marked(&mut app, "150") && !marked(&mut app, "100"),
        "the marked chip follows"
    );
}
