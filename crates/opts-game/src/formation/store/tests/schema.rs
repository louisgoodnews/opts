//! Formation file schema: the documented layout, the round trip, hand-written and damaged files, and the car limit.

use super::*;

#[test]
fn the_schema_is_the_documented_one() {
    let text = encode(&sample()).expect("encodes");
    assert!(text.contains("format = 1"), "{text}");
    assert!(text.contains("name = \"Three NF6D\""));
    // The writer uses a TOML literal string (no escapes); the reader takes either form.
    assert!(
        text.contains("template = 'Trains\\X10_112_IC-8.zug'"),
        "{text}"
    );
    assert!(decode("format = 1\nname = \"x\"\ntemplate = \"Trains\\\\a.zug\"\n").is_ok());
    assert_eq!(text.matches("[[segment]]").count(), 3);
    assert!(text.contains("orientation = \"backward\""));
    assert!(text.contains("group = \"M-Train Cars\""));
    assert!(!text.contains("powered = \n"), "absent when automatic");
}

#[test]
fn a_formation_survives_the_round_trip_exactly() {
    let f = sample();
    assert_eq!(decode(&encode(&f).expect("encodes")).expect("decodes"), f);
    let custom = TrainFormation::custom("empty");
    assert_eq!(decode(&encode(&custom).expect("e")).expect("d"), custom);
}

#[test]
fn hand_written_files_load_with_defaults() {
    let f = decode("format = 1\nname = \"x\"\n\n[[segment]]\nvehicle = \"a.ovh\"\n\n[[segment]]\nvehicle = \"b.ovh\"\norientation = \"Backward\"\n")
        .expect("decodes");
    assert_eq!(f.segments[0].orientation, Orientation::Forward);
    assert_eq!(f.segments[1].orientation, Orientation::Backward);
    assert_eq!(f.source, FormationSource::Custom);
}

#[test]
fn bad_files_say_what_is_wrong() {
    for (text, needle) in [
        ("not toml [", "expected"),
        ("name = \"x\"\n", "format"),
        ("format = 0\nname = \"x\"\n", "format must be 1"),
        ("format = 1\nname = \"x\"\n[[segment]]\n", "exactly one"),
        (
            "format = 1\nname = \"x\"\n[[segment]]\nvehicle = \"a\"\ngroup = \"b\"\n",
            "exactly one",
        ),
        (
            "format = 1\nname = \"x\"\n[[segment]]\nvehicle = \"a\"\norientation = \"up\"\n",
            "orientation",
        ),
    ] {
        let e = decode(text).expect_err(text).to_string();
        assert!(e.to_lowercase().contains(needle), "{text:?} -> {e}");
    }
    assert_eq!(
        decode("format = 9\nname = \"x\"\n"),
        Err(StoreError::Newer(9))
    );
}

#[test]
fn too_many_cars_are_refused() {
    let mut t = String::from("format = 1\nname = \"x\"\n");
    for _ in 0..=MAX_CARS {
        t.push_str("[[segment]]\nvehicle = \"a.ovh\"\n");
    }
    assert!(matches!(decode(&t), Err(StoreError::Corrupt(m)) if m.contains("more than")));
}
