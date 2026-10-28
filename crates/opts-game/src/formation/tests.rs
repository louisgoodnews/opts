//! Unit tests for `super` (`formation.rs`).

use super::*;

fn from(params: Value) -> Result<TrainFormation, RpcError> {
    formation_from(&World::new(), &params)
}

#[test]
fn cars_may_be_strings_or_objects_and_default_to_forward_unspecified_power() {
    let f = from(json!({
        "name": "Mine",
        "cars": [
            "vehicles\\A\\loco.ovh",
            {"vehicle": "vehicles\\A\\wagon.ovh", "orientation": "backward", "powered": false},
            {"vehicle": "Some Group"}
        ]
    }))
    .expect("parses");
    assert_eq!(f.name, "Mine");
    assert_eq!(f.source, FormationSource::Custom);
    assert_eq!(f.segments[0].orientation, Orientation::Forward);
    assert_eq!(f.segments[0].powered, None);
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
    assert_eq!(f.segments[1].powered, Some(false));
    assert!(matches!(f.segments[2].vehicle, VehicleRef::Group(_)));
}

#[test]
fn reverse_turns_the_train_around() {
    let f =
        from(json!({"cars": ["a.ovh", {"vehicle": "b.ovh", "orientation": "1"}], "reverse": true}))
            .expect("parses");
    assert_eq!(f.segments[0].vehicle.as_str(), "b.ovh");
    assert_eq!(f.segments[0].orientation, Orientation::Forward);
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
}

#[test]
fn bad_parameters_are_reported_as_invalid_params() {
    for params in [
        json!({}),
        json!({"cars": "a.ovh"}),
        json!({"cars": [5]}),
        json!({"cars": [{"orientation": "forward"}]}),
        json!({"cars": [{"vehicle": "a.ovh", "orientation": "sideways"}]}),
    ] {
        let e = from(params.clone()).expect_err(&params.to_string());
        assert_eq!(e.code, code::INVALID_PARAMS, "{params}");
    }
}

#[test]
fn a_zug_needs_a_running_session() {
    let e = from(json!({"zug": "Trains\\x.zug"})).expect_err("no session");
    assert_eq!(e.code, code::NOT_READY);
}

#[test]
fn issues_are_listed_with_key_car_and_message() {
    let issues = [
        FormationIssue::TooShort,
        FormationIssue::NotRail { index: 2 },
    ];
    let v = issues_json(&issues);
    assert_eq!(v[0]["key"], "formation-err-too-short");
    assert!(v[0]["car"].is_null());
    assert_eq!(v[1]["car"], 2);
    assert!(v[1]["message"]
        .as_str()
        .is_some_and(|m| m.contains("car 3")));
}
