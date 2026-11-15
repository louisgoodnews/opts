//! Unit tests of the setvar kinds, the documented ranges and the offered names.

use super::*;

#[test]
fn the_kind_follows_the_values_the_repaints_give() {
    assert_eq!(kind_of(&["1"]).kind, SetvarKind::Switch, "only on");
    assert_eq!(kind_of(&["0"]).kind, SetvarKind::Switch);
    assert_eq!(kind_of(&["0", "1"]).kind, SetvarKind::Switch);
    let one_based = kind_of(&["1", "2", "3", "4"]);
    assert_eq!(one_based.kind, SetvarKind::Choice);
    assert_eq!(one_based.values, vec![1, 2, 3, 4]);
    assert_eq!((one_based.min, one_based.max), (1.0, 4.0));
    let coded = kind_of(&["0", "1", "10", "100"]);
    assert_eq!(
        coded.kind,
        SetvarKind::Choice,
        "codes with gaps are a choice"
    );
    assert_eq!(coded.values, vec![0, 1, 10, 100]);
    let number = kind_of(&["3964", "121"]);
    assert_eq!(number.kind, SetvarKind::Number);
    assert!(number.max >= NUMBER_MAX);
    let fraction = kind_of(&["-0.7", "-0,9", "0"]);
    assert_eq!(fraction.kind, SetvarKind::Fraction);
    assert_eq!((fraction.min, fraction.max), (-0.9, 0.0));
}

#[test]
fn a_documented_range_widens_the_choice_and_flags_a_narrower_document() {
    let family = CuratedFamily {
        id: "f".into(),
        title: "F".into(),
        folders: vec![],
        setvars: vec![
            CuratedSetvar {
                name: "ibis".into(),
                values: "0-2".into(),
                meaning: "IBIS language".into(),
                group: "display".into(),
                respawn: None,
            },
            CuratedSetvar {
                name: "wheels".into(),
                values: "0|1".into(),
                meaning: String::new(),
                group: String::new(),
                respawn: None,
            },
        ],
    };
    let observed = [
        sv("a", "ibis", "1"),
        sv("b", "ibis", "2"),
        sv("a", "wheels", "2"),
    ];
    let c = SetvarCatalogue::build(&observed, &BTreeMap::new(), Some(&family));
    let ibis = c.get("IBIS").expect("ibis");
    assert_eq!(ibis.kind, SetvarKind::Choice);
    assert_eq!(
        ibis.values,
        vec![0, 1, 2],
        "0 is documented, repaints never set it"
    );
    assert!(!ibis.range_unverified);
    assert_eq!(ibis.label, "IBIS language");
    assert_eq!(ibis.group.as_deref(), Some("display"));
    let wheels = c.get("wheels").expect("wheels");
    assert_eq!(
        wheels.values,
        vec![0, 1, 2],
        "the join of the document and the repaints"
    );
    assert!(
        wheels.range_unverified,
        "the repaints set 2, the document says 0|1"
    );
    assert_eq!(wheels.label, "Wheels");
}

#[test]
fn only_declared_names_are_offered_when_the_declarations_are_known() {
    let observed = [sv("a", "vis_a", "1"), sv("a", "vis_b", "1")];
    let lists = declared(&[("vis_a", "setvar_list.txt")]);
    let c = SetvarCatalogue::build(&observed, &lists, None);
    assert_eq!(c.entries.len(), 1);
    assert_eq!(c.entries[0].name, "vis_a");
    assert_eq!(
        c.entries[0].declared_in,
        vec!["setvar_list.txt".to_string()]
    );
    assert!(c.entries[0].evidence.declared);
    assert_eq!(c.undeclared, vec!["vis_b".to_string()]);
    // Unknown declarations offer everything.
    let all = SetvarCatalogue::build(&observed, &BTreeMap::new(), None);
    assert_eq!(all.entries.len(), 2);
    assert!(all.undeclared.is_empty());
}
