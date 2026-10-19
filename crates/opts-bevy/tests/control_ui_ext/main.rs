//! `ui.scrollables`, `ui.scroll`, `ui.toggle`, `ui.select`, `ui.text`, `ui.key`, `ui.click count`
//! against a headless app with real UI layout: every view is scrolled and every control operated
//! by id, through the same messages a person causes.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use bevy::prelude::*;
use opts_bevy::control::{ControlAddr, ControlConfig, ControlPlugin};
use opts_cli::client::{Client, ClientConfig, ClientError};
use opts_cli::protocol::code;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::combobox::ComboboxBuilder;
use opts_gui::components::combobox::{ComboboxHeader, M3Combobox};
use opts_gui::components::modal::ModalBuilder;
use opts_gui::components::switch::SwitchBuilder;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::testing::layout_app;
use opts_gui::ui_id::UiId;
use serde_json::{json, Value};

mod fixture;
use fixture::{fixture, rpc, settle, with_client};

#[test]
fn scrollables_list_the_view_with_its_items_and_limits() {
    let mut f = fixture();
    let v = with_client(&mut f.app, f.port, |c| {
        c.call("ui.scrollables", json!({})).expect("scrollables")
    });
    let list = v["scrollables"].as_array().expect("list");
    let view = list.iter().find(|s| s["id"] == "list.view").expect("view");
    assert_eq!(view["kind"], "container");
    assert!(
        view["max"][1].as_f64().unwrap() > 500.0,
        "30 rows overflow 200 px: {view}"
    );
    assert_eq!(view["at_top"], true);
    assert_eq!(view["at_bottom"], false);
    let items = view["items"].as_array().expect("items");
    assert!(
        items.iter().any(|i| i["id"] == "row.29"),
        "off-screen rows are listed too"
    );
}

#[test]
fn scroll_by_pages_to_ends_and_to_an_item() {
    let mut f = fixture();
    let (down, bottom, back, item, page) = with_client(&mut f.app, f.port, |c| {
        let down = c
            .call("ui.scroll", json!({"target": "list.view", "by": 120}))
            .expect("by");
        settle(40);
        let bottom = c
            .call("ui.scroll", json!({"target": "list.view", "to": "bottom"}))
            .expect("bottom");
        settle(40);
        let back = c
            .call("ui.scroll", json!({"target": "list.view", "to": "top"}))
            .expect("top");
        settle(60);
        let item = c
            .call(
                "ui.scroll",
                json!({"target": "list.view", "item": "row.20"}),
            )
            .expect("item");
        settle(60);
        let page = c
            .call("ui.scroll", json!({"target": "list.view", "pages": -0.5}))
            .expect("pages");
        (down, bottom, back, item, page)
    });
    assert!(
        (down["offset"][1].as_f64().unwrap() - 120.0).abs() < 1.0,
        "{down}"
    );
    assert_eq!(bottom["at_bottom"], true);
    assert_eq!(back["at_top"], true);
    let max = bottom["max"][1].as_f64().unwrap();
    let off = item["offset"][1].as_f64().unwrap();
    assert!(off > 400.0 && off <= max, "row.20 is far down: {item}");
    assert!(
        page["offset"][1].as_f64().unwrap() < off,
        "half a page up moves up: {page}"
    );
}

#[test]
fn scroll_validates_its_arguments_and_names_valid_targets() {
    let mut f = fixture();
    let (none, two, bad_to, unknown, unknown_item) = with_client(&mut f.app, f.port, |c| {
        (
            c.call("ui.scroll", json!({"target": "list.view"}))
                .expect_err("none"),
            c.call("ui.scroll", json!({"by": 1, "to": "top"}))
                .expect_err("two"),
            c.call("ui.scroll", json!({"to": "middle"}))
                .expect_err("to"),
            c.call("ui.scroll", json!({"target": "nope", "by": 1}))
                .expect_err("unknown"),
            c.call(
                "ui.scroll",
                json!({"target": "list.view", "item": "row.99"}),
            )
            .expect_err("item"),
        )
    });
    assert_eq!(rpc(none).0, code::INVALID_PARAMS);
    assert_eq!(rpc(two).0, code::INVALID_PARAMS);
    assert_eq!(rpc(bad_to).0, code::INVALID_PARAMS);
    let (c, data) = rpc(unknown);
    assert_eq!(c, code::NOT_FOUND);
    assert!(data["valid"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "list.view"));
    assert_eq!(rpc(unknown_item).0, code::NOT_FOUND);
}

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

#[test]
fn select_chooses_a_combobox_option_by_text_or_index() {
    let mut f = fixture();
    let (by_text, by_index, bad) = with_client(&mut f.app, f.port, |c| {
        let by_text = c
            .call("ui.select", json!({"id": "opt.weather", "option": "rain"}))
            .expect("by text");
        settle(60);
        let by_index = c
            .call("ui.select", json!({"id": "opt.weather", "index": 2}))
            .expect("by index");
        let bad = c
            .call("ui.select", json!({"id": "opt.weather", "option": "Snow"}))
            .expect_err("no such option");
        (by_text, by_index, bad)
    });
    assert_eq!(by_text["selected"], "Rain");
    assert_eq!(by_index["selected"], "Fog");
    let (code_, data) = rpc(bad);
    assert_eq!(code_, code::NOT_FOUND);
    assert_eq!(data["options"], json!(["Sunny", "Rain", "Fog"]));
}

#[test]
fn a_long_dropdown_is_searched_with_text_and_chosen_with_select() {
    let mut f = fixture();
    let root = f
        .app
        .world_mut()
        .query::<(Entity, &opts_gui::components::combobox::M3Combobox)>()
        .iter(f.app.world())
        .find(|(_, c)| c.options.len() == 12)
        .map(|(e, _)| e)
        .expect("the long combobox");
    f.app
        .world_mut()
        .get_mut::<opts_gui::components::combobox::M3Combobox>(root)
        .expect("combobox")
        .is_open = true;
    for _ in 0..6 {
        f.app.update();
    }
    let (typed, picked) = with_client(&mut f.app, f.port, |c| {
        let typed = c
            .call(
                "ui.text",
                json!({
                    "id": "opt.start/textbox:widget-combobox-search-label",
                    "text": "point 11"
                }),
            )
            .expect("text into the search field");
        settle(400);
        let picked = c
            .call(
                "ui.select",
                json!({"id": "opt.start", "option": "Start point 11"}),
            )
            .expect("select");
        (typed, picked)
    });
    assert_eq!(typed["value"], "point 11");
    assert_eq!(picked["selected"], "Start point 11");
    let c = f
        .app
        .world()
        .get::<opts_gui::components::combobox::M3Combobox>(root)
        .expect("combobox");
    assert_eq!(c.selected, Some(11));
    assert!(!c.is_open);
}

#[test]
fn text_replaces_or_appends_in_a_field() {
    let mut f = fixture();
    let (a, b, c_) = with_client(&mut f.app, f.port, |c| {
        let a = c
            .call("ui.text", json!({"id": "opt.search", "text": "gla"}))
            .expect("text");
        settle(60);
        let b = c
            .call(
                "ui.text",
                json!({"id": "opt.search", "text": "sgow", "replace": false}),
            )
            .expect("append");
        settle(60);
        let c_ = c
            .call("ui.text", json!({"id": "opt.search", "text": "x"}))
            .expect("replace");
        (a, b, c_)
    });
    assert_eq!(a["value"], "gla");
    assert_eq!(b["value"], "glasgow");
    assert_eq!(c_["value"], "x");
}

#[test]
fn key_presses_modifiers_with_the_key() {
    let mut f = fixture();
    let ok = with_client(&mut f.app, f.port, |c| {
        c.call(
            "ui.key",
            json!({"key": "F", "ctrl": true, "duration_ms": 120}),
        )
        .expect("key")
    });
    assert_eq!(ok["modifiers"], json!(["ctrl"]));
    assert!(
        f.chord.load(Ordering::SeqCst),
        "Ctrl and F were held in the same frame"
    );
    let bad = with_client(&mut f.app, f.port, |c| {
        c.call("ui.key", json!({"key": "Hyper"}))
            .expect_err("bad key")
    });
    assert_eq!(rpc(bad).0, code::INVALID_PARAMS);
}

#[test]
fn hover_moves_the_pointer_to_the_element() {
    let mut f = fixture();
    let r = with_client(&mut f.app, f.port, |c| {
        c.call("ui.hover", json!({"id": "row.0"})).expect("hover")
    });
    assert_eq!(r["ok"], true);
    assert!(r["pos"][0].as_f64().unwrap() > 0.0);
    let missing = with_client(&mut f.app, f.port, |c| {
        c.call("ui.hover", json!({"id": "nope"}))
            .expect_err("missing")
    });
    assert_eq!(rpc(missing).0, code::NOT_FOUND);
}

#[test]
fn a_windowed_list_lists_all_entries_scrolls_to_one_and_selects_it() {
    let mut f = fixture();
    let (before, moved, selected) = with_client(&mut f.app, f.port, |c| {
        let before = c.call("ui.scrollables", json!({})).expect("scrollables");
        let moved = c
            .call("ui.scroll", json!({"target": "maps.list", "item": "id250"}))
            .expect("scroll to entry");
        settle(60);
        let selected = c
            .call(
                "ui.select",
                json!({"id": "maps.list", "option": "Entry 00042"}),
            )
            .expect("select by title");
        (before, moved, selected)
    });
    let list = before["scrollables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == "maps.list")
        .expect("list is scrollable");
    assert_eq!(list["kind"], "list");
    assert_eq!(
        list["items"].as_array().unwrap().len(),
        300,
        "every entry, not only live rows"
    );
    assert!(moved["offset"][1].as_f64().unwrap() > 1000.0, "{moved}");
    assert_eq!(selected["selected"], "id42");
}
