//! Risky changes: applied at once, reverted when nobody confirms, saved when kept or reverted by hand.

use super::*;

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
