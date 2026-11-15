//! Modal gating, switches and click counts.

use std::sync::atomic::Ordering;

use bevy::prelude::*;
use opts_cli::protocol::code;
use opts_gui::components::modal::ModalBuilder;
use serde_json::json;

use super::fixture::{fixture, rpc, settle, with_client};

#[test]
fn an_open_dialog_gates_scrolling_and_operating_what_is_behind_it() {
    let mut f = fixture();
    {
        let mut commands = f.app.world_mut().commands();
        let (_root, content) = ModalBuilder::new().spawn(&mut commands);
        commands.entity(content).with_children(|p| {
            p.spawn(Text::new("Sure?"));
        });
    }
    for _ in 0..3 {
        f.app.update();
    }
    let (scroll, toggle, click, ok_error_code) = with_client(&mut f.app, f.port, |c| {
        (
            c.call("ui.scroll", json!({"target": "list.view", "by": 10}))
                .expect_err("scroll"),
            c.call("ui.toggle", json!({"id": "opt.switch"}))
                .expect_err("toggle"),
            c.call("ui.click", json!({"id": "row.0"}))
                .expect_err("click"),
            c.call("ui.scroll", json!({"target": "list.view", "by": 10}))
                .is_err(),
        )
    });
    assert_eq!(rpc(scroll).0, code::GATED);
    assert_eq!(rpc(toggle).0, code::GATED);
    assert_eq!(rpc(click).0, code::GATED);
    assert!(ok_error_code);
}

#[test]
fn toggle_sets_a_switch_idempotently_and_click_count_repeats() {
    let mut f = fixture();
    let (first, again, off, clicked) = with_client(&mut f.app, f.port, |c| {
        let first = c
            .call("ui.toggle", json!({"id": "opt.switch", "value": true}))
            .expect("on");
        settle(60);
        let again = c
            .call("ui.toggle", json!({"id": "opt.switch", "value": true}))
            .expect("again");
        let off = c
            .call("ui.toggle", json!({"id": "opt.switch"}))
            .expect("flip");
        settle(60);
        let clicked = c
            .call("ui.click", json!({"id": "row.0", "count": 3}))
            .expect("count");
        settle(80);
        (first, again, off, clicked)
    });
    assert_eq!(
        (&first["changed"], &first["checked"]),
        (&json!(true), &json!(true))
    );
    assert_eq!(again["changed"], false, "already on: nothing to do");
    assert_eq!(
        (&off["changed"], &off["checked"]),
        (&json!(true), &json!(false))
    );
    assert_eq!(clicked["count"], 3);
    assert_eq!(f.clicks.load(Ordering::SeqCst), 3);
}
