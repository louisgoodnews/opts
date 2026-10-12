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
use opts_gui::components::modal::ModalBuilder;
use opts_gui::components::switch::SwitchBuilder;
use opts_gui::components::text_field::TextFieldBuilder;
use opts_gui::testing::layout_app;
use opts_gui::ui_id::UiId;
use serde_json::{json, Value};

#[derive(Resource, Default, Clone)]
struct Chord(Arc<AtomicBool>);

fn watch_chord(keys: Res<ButtonInput<KeyCode>>, chord: Res<Chord>) {
    if keys.pressed(KeyCode::ControlLeft) && keys.pressed(KeyCode::KeyF) {
        chord.0.store(true, Ordering::SeqCst);
    }
}

/// A windowed list of 300 entries beside the other fixtures (the wizard's map list is one).
fn spawn_list(app: &mut App) -> Entity {
    use opts_gui::l10n::{wizard_keys::md, Localization};
    use opts_gui::patterns::list_model::ListEntry;
    use opts_gui::patterns::master_detail::{
        spawn_master_detail, MasterDetail, MasterDetailConfig,
    };
    let loc = app.world().resource::<Localization>().clone();
    let entries: Vec<ListEntry> = (0..300)
        .map(|i| ListEntry::new(format!("id{i}"), format!("Entry {i:05}")))
        .collect();
    let mut commands = app.world_mut().commands();
    let parts = spawn_master_detail(
        &mut commands,
        &loc,
        MasterDetailConfig {
            prefix: "maps",
            empty_key: md::NO_RESULTS,
            search_key: md::SEARCH,
        },
        MasterDetail::new(entries, None),
    );
    let frame = commands
        .spawn(Node {
            width: Val::Px(900.0),
            height: Val::Px(500.0),
            ..default()
        })
        .id();
    commands.entity(frame).add_child(parts.root);
    parts.root
}

struct Fixture {
    app: App,
    port: u16,
    clicks: Arc<AtomicUsize>,
    chord: Arc<AtomicBool>,
}

fn fixture() -> Fixture {
    let mut app = layout_app();
    app.add_plugins(ControlPlugin::new(ControlConfig {
        port: Some(0),
        ..ControlConfig::default()
    }));
    let chord = Chord::default();
    let seen = chord.0.clone();
    app.insert_resource(chord).add_systems(Update, watch_chord);
    let clicks = Arc::new(AtomicUsize::new(0));
    let c = clicks.clone();
    {
        let mut commands = app.world_mut().commands();
        // A 300 x 200 scroll view with 30 buttons of about 40 px.
        let view = commands
            .spawn((
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(200.0),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                ScrollPosition::default(),
                UiId::new("list.view"),
            ))
            .id();
        let rows: Vec<Entity> = (0..30)
            .map(|i| {
                let mut b = ButtonBuilder::filled(format!("Row {i}"));
                if i == 0 {
                    let c = c.clone();
                    b = b.on_click(move |_, _| {
                        c.fetch_add(1, Ordering::SeqCst);
                    });
                }
                let e = b.spawn(&mut commands);
                commands.entity(e).insert(UiId::new(format!("row.{i}")));
                e
            })
            .collect();
        commands.entity(view).add_children(&rows);
        let sw = SwitchBuilder::standalone().spawn(&mut commands);
        commands.entity(sw).insert(UiId::new("opt.switch"));
        let combo = ComboboxBuilder::new("Weather")
            .with_options(vec!["Sunny".into(), "Rain".into(), "Fog".into()])
            .with_selected(0)
            .spawn(&mut commands);
        let _ = combo;
        let field = TextFieldBuilder::outlined("Search").spawn(&mut commands);
        commands.entity(field).insert(UiId::new("opt.search"));
    }
    spawn_list(&mut app);
    for _ in 0..6 {
        app.update();
    }
    // The drop-down is operated through its header, the part that carries the id in the tree.
    let header = app
        .world_mut()
        .query::<(Entity, &opts_gui::components::combobox::ComboboxHeader)>()
        .iter(app.world())
        .next()
        .map(|(e, _)| e)
        .expect("combobox header");
    app.world_mut()
        .entity_mut(header)
        .insert(UiId::new("opt.weather"));
    app.update();
    let port = app.world().resource::<ControlAddr>().0.port();
    Fixture {
        app,
        port,
        clicks,
        chord: seen,
    }
}

fn with_client<R: Send + 'static>(
    app: &mut App,
    port: u16,
    f: impl FnOnce(&mut Client) -> R + Send + 'static,
) -> R {
    let done = Arc::new(AtomicBool::new(false));
    let flag = done.clone();
    let out = Arc::new(Mutex::new(None));
    let slot = out.clone();
    let handle = thread::spawn(move || {
        let mut c = Client::new(ClientConfig {
            host: "127.0.0.1".into(),
            port,
            token: None,
            timeout: Duration::from_secs(10),
        });
        // A failed expectation must end the pumping loop too, or the test would hang.
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut c)));
        *slot.lock().expect("slot") = Some(r);
        flag.store(true, Ordering::SeqCst);
    });
    while !done.load(Ordering::SeqCst) {
        app.update();
        thread::sleep(Duration::from_millis(4));
    }
    handle.join().expect("client thread");
    let r = out.lock().expect("slot").take().expect("result");
    match r {
        Ok(v) => v,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

fn rpc(e: ClientError) -> (i64, Value) {
    match e {
        ClientError::Rpc(r) => (r.code, r.data.unwrap_or(Value::Null)),
        other => panic!("expected an rpc error, got {other}"),
    }
}

fn settle(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}

#[test]
fn scrollables_list_the_view_with_its_items_and_limits() {
    let mut f = fixture();
    let v = with_client(&mut f.app, f.port, |c| {
        c.call("ui.scrollables", json!({})).expect("scrollables")
    });
    let list = v["scrollables"].as_array().expect("list");
    let view = list.iter().find(|s| s["id"] == "list.view").expect("view");
    assert_eq!(view["kind"], "container");
    assert!(view["max"][1].as_f64().unwrap() > 500.0, "30 rows overflow 200 px: {view}");
    assert_eq!(view["at_top"], true);
    assert_eq!(view["at_bottom"], false);
    let items = view["items"].as_array().expect("items");
    assert!(items.iter().any(|i| i["id"] == "row.29"), "off-screen rows are listed too");
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
            .call("ui.scroll", json!({"target": "list.view", "item": "row.20"}))
            .expect("item");
        settle(60);
        let page = c
            .call("ui.scroll", json!({"target": "list.view", "pages": -0.5}))
            .expect("pages");
        (down, bottom, back, item, page)
    });
    assert!((down["offset"][1].as_f64().unwrap() - 120.0).abs() < 1.0, "{down}");
    assert_eq!(bottom["at_bottom"], true);
    assert_eq!(back["at_top"], true);
    let max = bottom["max"][1].as_f64().unwrap();
    let off = item["offset"][1].as_f64().unwrap();
    assert!(off > 400.0 && off <= max, "row.20 is far down: {item}");
    assert!(page["offset"][1].as_f64().unwrap() < off, "half a page up moves up: {page}");
}

#[test]
fn scroll_validates_its_arguments_and_names_valid_targets() {
    let mut f = fixture();
    let (none, two, bad_to, unknown, unknown_item) = with_client(&mut f.app, f.port, |c| {
        (
            c.call("ui.scroll", json!({"target": "list.view"})).expect_err("none"),
            c.call("ui.scroll", json!({"by": 1, "to": "top"})).expect_err("two"),
            c.call("ui.scroll", json!({"to": "middle"})).expect_err("to"),
            c.call("ui.scroll", json!({"target": "nope", "by": 1})).expect_err("unknown"),
            c.call("ui.scroll", json!({"target": "list.view", "item": "row.99"})).expect_err("item"),
        )
    });
    assert_eq!(rpc(none).0, code::INVALID_PARAMS);
    assert_eq!(rpc(two).0, code::INVALID_PARAMS);
    assert_eq!(rpc(bad_to).0, code::INVALID_PARAMS);
    let (c, data) = rpc(unknown);
    assert_eq!(c, code::NOT_FOUND);
    assert!(data["valid"].as_array().unwrap().iter().any(|v| v == "list.view"));
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
            c.call("ui.scroll", json!({"target": "list.view", "by": 10})).expect_err("scroll"),
            c.call("ui.toggle", json!({"id": "opt.switch"})).expect_err("toggle"),
            c.call("ui.click", json!({"id": "row.0"})).expect_err("click"),
            c.call("ui.scroll", json!({"target": "list.view", "by": 10})).is_err(),
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
        let first = c.call("ui.toggle", json!({"id": "opt.switch", "value": true})).expect("on");
        settle(60);
        let again = c.call("ui.toggle", json!({"id": "opt.switch", "value": true})).expect("again");
        let off = c.call("ui.toggle", json!({"id": "opt.switch"})).expect("flip");
        settle(60);
        let clicked = c.call("ui.click", json!({"id": "row.0", "count": 3})).expect("count");
        settle(80);
        (first, again, off, clicked)
    });
    assert_eq!((&first["changed"], &first["checked"]), (&json!(true), &json!(true)));
    assert_eq!(again["changed"], false, "already on: nothing to do");
    assert_eq!((&off["changed"], &off["checked"]), (&json!(true), &json!(false)));
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
fn text_replaces_or_appends_in_a_field() {
    let mut f = fixture();
    let (a, b, c_) = with_client(&mut f.app, f.port, |c| {
        let a = c.call("ui.text", json!({"id": "opt.search", "text": "gla"})).expect("text");
        settle(60);
        let b = c
            .call("ui.text", json!({"id": "opt.search", "text": "sgow", "replace": false}))
            .expect("append");
        settle(60);
        let c_ = c.call("ui.text", json!({"id": "opt.search", "text": "x"})).expect("replace");
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
        c.call("ui.key", json!({"key": "F", "ctrl": true, "duration_ms": 120})).expect("key")
    });
    assert_eq!(ok["modifiers"], json!(["ctrl"]));
    assert!(f.chord.load(Ordering::SeqCst), "Ctrl and F were held in the same frame");
    let bad = with_client(&mut f.app, f.port, |c| {
        c.call("ui.key", json!({"key": "Hyper"})).expect_err("bad key")
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
        c.call("ui.hover", json!({"id": "nope"})).expect_err("missing")
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
            .call("ui.select", json!({"id": "maps.list", "option": "Entry 00042"}))
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
    assert_eq!(list["items"].as_array().unwrap().len(), 300, "every entry, not only live rows");
    assert!(moved["offset"][1].as_f64().unwrap() > 1000.0, "{moved}");
    assert_eq!(selected["selected"], "id42");
}
