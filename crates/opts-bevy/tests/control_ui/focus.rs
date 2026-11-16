//! Click, focus, action and state drive the UI.

use super::*;

#[test]
fn click_focus_action_and_state_drive_the_ui() {
    let (mut app, port, clicks) = app();
    let (after_focus, after_nav, click_ok) = with_client(&mut app, port, |c| {
        c.call("ui.focus", json!({"id": "menu.quit"}))
            .expect("focus");
        thread::sleep(Duration::from_millis(60));
        let focused = c
            .call("ui.state", json!({"id": "menu.quit"}))
            .expect("state");
        c.call("ui.action", json!({"action": "navigate", "arg": "up"}))
            .expect("action");
        thread::sleep(Duration::from_millis(60));
        let nav = c.call("ui.tree", Value::Null).expect("tree")["focus"].clone();
        let click = c
            .call("ui.click", json!({"id": "menu.start"}))
            .expect("click");
        thread::sleep(Duration::from_millis(60));
        (focused, nav, click)
    });
    assert_eq!(after_focus["state"]["focused"], true);
    assert_eq!(
        after_nav, "menu.start",
        "navigate up moves focus to the previous element"
    );
    assert_eq!(click_ok["ok"], true);
    assert_eq!(
        clicks.load(Ordering::SeqCst),
        1,
        "ui.click activated the button"
    );
}
