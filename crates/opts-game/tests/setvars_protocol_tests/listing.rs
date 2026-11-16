//! Integration tests for listing the setvars options of a vehicle and changing them.
use super::*;

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
