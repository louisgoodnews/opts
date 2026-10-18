//! `vehicle.setvars` (M-0078) in the wizard: listing the options of the chosen vehicle with kind,
//! range, value and default, changing them all or none, and the refusals.

use bevy::prelude::*;
use opts_bevy::control::channel::dispatch;
use opts_bevy::control::state::{ControlRegistry, Outcome};
use opts_cli::protocol::{code, RpcError};
use opts_game::new_game_screen::livery::LiveryState;
use opts_game::new_game_screen::repaints::VehicleRepaints;
use opts_game::new_game_screen::types::NewGameSetup;
use serde_json::{json, Value};

fn reps() -> VehicleRepaints {
    let mut reps = VehicleRepaints {
        default_name: "Red".into(),
        folder: "Citybus 530 by Kajosoft".into(),
        ..VehicleRepaints::default()
    };
    reps.setvars.insert(
        "Red".into(),
        vec![
            ("vis_x".into(), "1".into()),
            ("display_ext".into(), "2".into()),
            ("vehicle_id".into(), "3964".into()),
            ("language_ibis".into(), "1".into()),
        ],
    );
    reps.setvars
        .insert("Blue".into(), vec![("display_ext".into(), "5".into())]);
    reps
}

fn app(with_repaints: bool) -> App {
    let mut app = App::new();
    let mut registry = ControlRegistry::default();
    opts_game::control_setvars::register(&mut registry);
    app.insert_resource(registry)
        .insert_resource(NewGameSetup::default());
    let mut liveries = LiveryState::default();
    if with_repaints {
        liveries.repaints = Some(reps());
    }
    app.insert_resource(liveries);
    app
}

fn call(app: &mut App, params: Value) -> Result<Value, RpcError> {
    match dispatch(app.world_mut(), "vehicle.setvars", &params) {
        Outcome::Done(r) => r,
        Outcome::Wait(_) => panic!("answers at once"),
    }
}

fn option<'a>(list: &'a Value, name: &str) -> &'a Value {
    list["options"]
        .as_array()
        .and_then(|o| o.iter().find(|x| x["name"] == name))
        .unwrap_or_else(|| panic!("no option {name}"))
}

fn custom(app: &App, name: &str) -> Option<i32> {
    app.world()
        .resource::<NewGameSetup>()
        .custom_setvars
        .get(name)
        .copied()
}

#[test]
fn the_options_of_the_chosen_vehicle_are_listed_with_kind_range_value_and_default() {
    let mut app = app(true);
    let list = call(&mut app, json!({})).expect("options");
    assert_eq!(list["context"], "wizard");
    assert_eq!(list["repaint"], "Red");
    assert_eq!(list["family"], "citybus-o530-kajosoft");
    let sw = option(&list, "vis_x");
    assert_eq!(
        (
            sw["kind"].as_str(),
            sw["value"].as_f64(),
            sw["default"].as_f64()
        ),
        (Some("switch"), Some(1.0), Some(1.0))
    );
    let display = option(&list, "display_ext");
    assert_eq!(display["kind"], "choice");
    assert_eq!(display["values"], json!([2, 5]));
    assert_eq!(display["evidence"]["repaints"], true);
    let id = option(&list, "vehicle_id");
    assert_eq!(id["kind"], "number");
    assert!(id["max"].as_f64().unwrap_or(0.0) >= 999_999.0);
    // The documented family gives group and meaning, and widens the range with the document's.
    let ibis = option(&list, "language_ibis");
    assert_eq!(ibis["group"], "display");
    assert_eq!(ibis["values"], json!([0, 1, 2]));
    assert_eq!(ibis["evidence"]["documented"], true);
    // The chosen repaint decides the defaults.
    app.world_mut()
        .resource_mut::<NewGameSetup>()
        .selected_livery = "Blue".into();
    let blue = call(&mut app, json!({})).expect("options");
    assert_eq!(option(&blue, "display_ext")["default"].as_f64(), Some(5.0));
    assert_eq!(option(&blue, "vis_x")["default"].as_f64(), Some(0.0));
}

#[test]
fn a_change_is_a_choice_of_the_plan_and_the_default_removes_it() {
    let mut app = app(true);
    let done = call(
        &mut app,
        json!({ "values": { "display_ext": 5, "vehicle_id": "100", "vis_x": false } }),
    )
    .expect("set");
    assert_eq!(done["count"], 3);
    assert_eq!(done["live"], false);
    assert_eq!(
        (
            custom(&app, "display_ext"),
            custom(&app, "vehicle_id"),
            custom(&app, "vis_x")
        ),
        (Some(5), Some(100), Some(0))
    );
    let list = call(&mut app, json!({})).expect("options");
    assert_eq!(option(&list, "display_ext")["value"].as_f64(), Some(5.0));
    assert_eq!(option(&list, "display_ext")["default"].as_f64(), Some(2.0));
    // Back to the repaint's own values: no override left.
    call(
        &mut app,
        json!({ "values": { "display_ext": 2, "vis_x": 1 } }),
    )
    .expect("set");
    assert_eq!(
        (custom(&app, "display_ext"), custom(&app, "vis_x")),
        (None, None)
    );
    assert_eq!(custom(&app, "vehicle_id"), Some(100));
}

#[test]
fn a_batch_is_applied_whole_or_not_at_all_and_every_refusal_is_listed() {
    let mut app = app(true);
    let bad = call(&mut app, json!({ "values": { "display_ext": 3, "no_such_option": 1, "vis_x": 7, "vehicle_id": 2.5 } })).expect_err("refused");
    assert_eq!(bad.code, code::INVALID_PARAMS);
    let errors = bad
        .data
        .as_ref()
        .and_then(|d| d["errors"].as_array())
        .expect("errors");
    let keys: Vec<&str> = errors.iter().filter_map(|e| e["key"].as_str()).collect();
    assert_eq!(keys.len(), 4, "{errors:?}");
    assert!(
        keys.contains(&"display_ext")
            && keys.contains(&"no_such_option")
            && keys.contains(&"vis_x")
            && keys.contains(&"vehicle_id")
    );
    assert!(
        app.world()
            .resource::<NewGameSetup>()
            .custom_setvars
            .is_empty(),
        "nothing changed"
    );
    assert_eq!(
        call(&mut app, json!({ "values": {} }))
            .expect_err("empty")
            .code,
        code::INVALID_PARAMS
    );
    assert_eq!(
        call(&mut app, json!({ "values": 3 }))
            .expect_err("not an object")
            .code,
        code::INVALID_PARAMS
    );
}

#[test]
fn an_unlisted_name_needs_allow_unlisted() {
    let mut app = app(true);
    assert!(call(&mut app, json!({ "values": { "cp_extra": 1 } })).is_err());
    let done = call(
        &mut app,
        json!({ "values": { "cp_extra": 1 }, "allow_unlisted": true }),
    )
    .expect("allowed");
    assert_eq!(done["count"], 1);
    assert_eq!(custom(&app, "cp_extra"), Some(1));
}

#[test]
fn without_a_vehicle_to_configure_it_says_not_ready() {
    let mut app = app(false);
    assert_eq!(
        call(&mut app, json!({})).expect_err("no vehicle").code,
        code::NOT_READY
    );
}
