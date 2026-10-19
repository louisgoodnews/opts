//! Choosing, resetting, searching and risky changes.

use super::*;

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
    let (mut app, _, _) = page_app("notyet", "audio");
    let b = by_id(&mut app, "settings/audio/audio.master/down").expect("stepper stays visible");
    assert!(app.world().get::<Disabled>(b).is_some());
    let labels = app
        .world_mut()
        .query_filtered::<(), With<opts_gui::not_yet::NotYetLabel>>()
        .iter(app.world())
        .count();
    assert_eq!(labels, 4, "one red label per planned audio row");
}

fn advance(app: &mut App, secs: u32) {
    for _ in 0..secs {
        app.update();
    }
}

#[test]
fn a_risky_change_reverts_when_nobody_confirms() {
    let (mut app, log, db) = page_app("revert", "graphics");
    let b = by_id(
        &mut app,
        "settings/graphics/graphics.window_mode/option-borderless",
    )
    .expect("option");
    click(&mut app, b);
    app.update();
    assert_eq!(
        text(&app, "graphics.window_mode"),
        "borderless",
        "applied at once"
    );
    assert!(
        by_id(&mut app, "settings/revert-prompt/keep").is_some(),
        "prompt is open"
    );
    assert_eq!(
        stored(&db, "graphics.window_mode"),
        None,
        "not saved before it is kept"
    );
    advance(&mut app, 16);
    assert_eq!(
        text(&app, "graphics.window_mode"),
        "windowed",
        "15 s without an answer: back"
    );
    assert!(
        by_id(&mut app, "settings/revert-prompt/keep").is_none(),
        "prompt closed"
    );
    let log = log.lock().expect("log").clone();
    assert_eq!(
        log,
        vec![
            ("graphics.window_mode".to_string(), "borderless".to_string()),
            ("graphics.window_mode".to_string(), "windowed".to_string()),
        ]
    );
    assert_eq!(stored(&db, "graphics.window_mode"), None);
}

#[test]
fn keeping_a_risky_change_saves_it() {
    let (mut app, _, db) = page_app("keep", "graphics");
    let b = by_id(
        &mut app,
        "settings/graphics/graphics.resolution/option-1280x720",
    )
    .expect("option");
    click(&mut app, b);
    app.update();
    let keep = by_id(&mut app, "settings/revert-prompt/keep").expect("prompt");
    click(&mut app, keep);
    app.update();
    app.update();
    assert_eq!(text(&app, "graphics.resolution"), "1280x720");
    assert_eq!(
        stored(&db, "graphics.resolution").as_deref(),
        Some("1280x720")
    );
    advance(&mut app, 20);
    assert_eq!(
        text(&app, "graphics.resolution"),
        "1280x720",
        "no late revert"
    );
}

#[test]
fn going_back_returns_the_old_value() {
    let (mut app, _, _) = page_app("goback", "graphics");
    let b = by_id(
        &mut app,
        "settings/graphics/graphics.resolution/option-1600x900",
    )
    .expect("option");
    click(&mut app, b);
    app.update();
    let back = by_id(&mut app, "settings/revert-prompt/revert").expect("prompt");
    click(&mut app, back);
    app.update();
    app.update();
    assert_eq!(text(&app, "graphics.resolution"), "1920x1080");
}
