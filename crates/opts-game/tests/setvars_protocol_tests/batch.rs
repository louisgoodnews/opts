use super::*;

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
