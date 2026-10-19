//! Unit tests of `setvars`.

use super::*;

fn sv(repaint: &str, var: &str, value: &str) -> CtiSetvar {
    CtiSetvar {
        repaint: repaint.into(),
        variable: var.into(),
        value: value.into(),
    }
}

fn declared(names: &[(&str, &str)]) -> BTreeMap<String, BTreeSet<String>> {
    let mut m: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (n, list) in names {
        m.entry(n.to_lowercase())
            .or_default()
            .insert((*list).into());
    }
    m
}

fn kind_of(values: &[&str]) -> SetvarEntry {
    let observed: Vec<CtiSetvar> = values
        .iter()
        .enumerate()
        .map(|(i, v)| sv(&format!("r{i}"), "opt", v))
        .collect();
    let c = SetvarCatalogue::build(&observed, &BTreeMap::new(), None);
    c.entries.into_iter().next().expect("one entry")
}

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
            },
            CuratedSetvar {
                name: "wheels".into(),
                values: "0|1".into(),
                meaning: String::new(),
                group: String::new(),
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

#[test]
fn the_shipped_data_file_is_sound() {
    let families = curated_families();
    assert_eq!(families.len(), 12, "twelve documented families");
    let mut ids = BTreeSet::new();
    for f in families {
        assert!(ids.insert(&f.id), "duplicate family id {}", f.id);
        assert!(!f.setvars.is_empty(), "{}: no setvars", f.id);
        let mut names = BTreeSet::new();
        for s in &f.setvars {
            assert_eq!(
                s.name,
                s.name.trim(),
                "{}: name not trimmed: {:?}",
                f.id,
                s.name
            );
            assert!(
                names.insert(s.name.to_lowercase()),
                "{}: duplicate {}",
                f.id,
                s.name
            );
        }
    }
    let total: usize = families.iter().map(|f| f.setvars.len()).sum();
    assert_eq!(total, 510);
    let o530 = family_of_folder("citybus 530 by kajosoft").expect("matched case-insensitively");
    assert_eq!(o530.setvars.len(), 83);
    assert!(family_of_folder("No Such Folder").is_none());
    // Plain ranges dominate; the rest is free text that gives no documented range.
    let plain = families
        .iter()
        .flat_map(|f| &f.setvars)
        .filter(|s| parse_range(&s.values).is_some())
        .count();
    assert!(plain > 400, "only {plain} plain ranges");
}
