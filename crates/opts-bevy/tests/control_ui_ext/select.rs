//! Combo boxes, the windowed list, text fields, key presses and hover.


use bevy::prelude::*;
use opts_cli::protocol::code;
use serde_json::json;

use super::fixture::{fixture, rpc, settle, with_client};

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
