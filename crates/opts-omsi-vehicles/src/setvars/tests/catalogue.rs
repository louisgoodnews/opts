//! Unit tests of the labels, the documented value ranges and the catalogue merge.

use super::*;

#[test]
fn a_documented_name_no_repaint_sets_is_offered_when_the_vehicle_declares_it() {
    let family = CuratedFamily {
        id: "f".into(),
        title: "F".into(),
        folders: vec![],
        setvars: vec![CuratedSetvar {
            name: "ramp".into(),
            values: "0|1".into(),
            meaning: "ramp variant".into(),
            group: "doors".into(),
            respawn: None,
        }],
    };
    let c = SetvarCatalogue::build(
        &[],
        &declared(&[("ramp", "setvar_list.txt")]),
        Some(&family),
    );
    let e = c.get("ramp").expect("offered");
    assert_eq!(e.kind, SetvarKind::Switch);
    assert!(e.evidence.documented && e.evidence.declared && !e.evidence.observed);
    let none = SetvarCatalogue::build(&[], &declared(&[("other", "x.txt")]), Some(&family));
    assert!(none.entries.is_empty(), "not declared by this vehicle");
}

#[test]
fn the_default_is_the_value_of_the_chosen_repaint() {
    let observed = [
        sv("Red", "air_conditioning", "2"),
        sv("Blue", "air_conditioning", "4"),
        sv("Red", "mirror_right", "1"),
    ];
    let c = SetvarCatalogue::build(&observed, &BTreeMap::new(), None);
    let ac = c.get("air_conditioning").expect("ac");
    assert_eq!(ac.default_for("Red"), 2.0);
    assert_eq!(ac.default_for("Blue"), 4.0);
    assert_eq!(ac.default_for("Green"), 0.0, "not set by that repaint");
    assert!(
        ac.accepts(2.0) && ac.accepts(4.0) && !ac.accepts(3.0),
        "a choice accepts its values"
    );
    assert!(c
        .get("mirror_right")
        .is_some_and(|m| m.accepts(0.0) && !m.accepts(2.0)));
}

#[test]
fn the_names_are_matched_without_regard_to_case_and_keep_their_first_spelling() {
    let observed = [sv("a", "Vis_Front", "1"), sv("b", "vis_front", "1")];
    let c = SetvarCatalogue::build(&observed, &declared(&[("VIS_FRONT", "l.txt")]), None);
    assert_eq!(c.entries.len(), 1);
    assert_eq!(c.entries[0].name, "Vis_Front");
}

#[test]
fn labels_use_a_short_meaning_and_demote_a_long_one_to_the_description() {
    let long = CuratedSetvar {
        name: "air_conditioning_2".into(),
        values: "0-3".into(),
        meaning: "second A/C unit; only on articulated versions (unit on trailer)".into(),
        group: "climate".into(),
        respawn: None,
    };
    let (label, description) = labels("air_conditioning_2", Some(&long));
    assert_eq!(label, "Air conditioning 2");
    assert!(description.starts_with("Second A/C unit"));
    assert_eq!(humanise("vis_front_light"), "Vis front light");
}

#[test]
fn documented_ranges_parse_only_when_they_are_plain() {
    assert_eq!(parse_range("0|1"), Some([0, 1].into()));
    assert_eq!(parse_range("0-3"), Some([0, 1, 2, 3].into()));
    assert_eq!(parse_range("0,1,10"), Some([0, 1, 10].into()));
    for text in [
        "list",
        "not stated",
        "LINE 0|1 ; CITY 0-2",
        "number (any, suggested 0-9999)",
        "",
    ] {
        assert_eq!(parse_range(text), None, "{text:?}");
    }
}
