//! Scrolling of the list view: the scrollables listing, paging, item targets and argument errors.


use opts_cli::protocol::code;
use serde_json::json;

use super::fixture::{fixture, rpc, settle, with_client};

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
