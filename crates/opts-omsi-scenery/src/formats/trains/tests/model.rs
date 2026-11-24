//! Unit tests of the train formation parser and the formation model.

use super::*;

#[test]
fn text_round_trips_and_the_model_serialises() {
    let f = ok("a.ovh\r\n0\r\nb.ovh\r\n1\r\nGroup X\r\n0");
    let (again, _) =
        TrainFormation::from_zug(f.to_zug_text().as_bytes(), "t", "Trains\\t.zug").expect("again");
    assert_eq!(again, f);
    let json = serde_json::to_string(&f).expect("json");
    let back: TrainFormation = serde_json::from_str(&json).expect("from json");
    assert_eq!(back, f);
}

#[test]
fn orientation_helpers() {
    assert_eq!(Orientation::Forward.zug_value(), 0);
    assert_eq!(Orientation::Backward.zug_value(), 1);
    assert_eq!(Orientation::Forward.flipped(), Orientation::Backward);
    assert_eq!(Orientation::default(), Orientation::Forward);
}

#[test]
fn a_custom_formation_starts_empty() {
    let f = TrainFormation::custom("Mine");
    assert!(f.is_empty());
    assert_eq!(f.source, FormationSource::Custom);
}

// ---- validation -------------------------------------------------------------------------------

#[test]
fn powered_is_optional_in_the_serialised_form_and_zug_never_sets_it() {
    let f = ok("a.ovh\r\n0\r\nb.ovh\r\n1");
    assert!(f.segments.iter().all(|s| s.powered.is_none()));
    let json = serde_json::to_string(&f).expect("json");
    assert!(!json.contains("powered"), "{json}");
    let mut g = f.clone();
    g.segments[1].powered = Some(true);
    let back: TrainFormation =
        serde_json::from_str(&serde_json::to_string(&g).expect("json")).expect("back");
    assert_eq!(back.segments[1].powered, Some(true));
    // An older file without the field loads.
    let old = r#"{"name":"o","segments":[{"vehicle":{"File":"a.ovh"},"orientation":"forward"}],"source":"Custom"}"#;
    let o: TrainFormation = serde_json::from_str(old).expect("old");
    assert_eq!(o.segments[0].powered, None);
}
