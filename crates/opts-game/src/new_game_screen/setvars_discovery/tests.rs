use std::collections::BTreeSet;

use super::*;

fn reps(lines: &[(&str, &str, &str)]) -> VehicleRepaints {
    let mut r = VehicleRepaints::default();
    for (repaint, var, value) in lines {
        r.setvars
            .entry((*repaint).to_string())
            .or_default()
            .push(((*var).to_string(), (*value).to_string()));
    }
    r
}

fn item<'a>(items: &'a [SetvarItem], name: &str) -> &'a SetvarItem {
    items.iter().find(|i| i.name == name).expect("item")
}

#[test]
fn unknown_options_are_named_by_their_own_name_not_by_a_generic_phrase() {
    assert_eq!(humanise("vis_front_light"), "Vis front light");
    let r = reps(&[
        ("Beige", "vis_front_light", "1"),
        ("Beige", "ident_visible", "0"),
    ]);
    let items = setvars_from(
        &r,
        &HashMap::from([("vis_front_light".to_string(), 0)]),
        "Beige",
    );
    assert_eq!(items.len(), 2);
    let front = item(&items, "vis_front_light");
    assert_eq!(
        (front.label.as_str(), front.value, front.default_value),
        ("Vis front light", 0, 1)
    );
    let ident = item(&items, "ident_visible");
    assert_ne!(ident.label, "Vehicle Equipment Option");
    assert!(!ident.label.contains('⟦'), "known options have a message");
}

#[test]
fn choices_keep_their_values_and_numbers_their_range() {
    let r = reps(&[
        ("Red", "display_ext", "2"),
        ("Blue", "display_ext", "5"),
        ("Red", "vehicle_id", "3964"),
    ]);
    let items = setvars_from(&r, &HashMap::new(), "Red");
    let display = item(&items, "display_ext");
    assert_eq!(display.kind, SetvarKind::Choice);
    assert_eq!(display.values, vec![2, 5], "what repaints give; Red sets 2");
    assert_eq!((display.value, display.default_value), (2, 2));
    // A repaint that leaves the option unset starts at 0, which is then listed as well.
    let green = setvars_from(&r, &HashMap::new(), "Green");
    let unset = item(&green, "display_ext");
    assert_eq!((unset.value, unset.default_value), (0, 0));
    assert_eq!(unset.values, vec![0, 2, 5]);
    let id = item(&items, "vehicle_id");
    assert_eq!(id.kind, SetvarKind::Number);
    assert_eq!(id.value, 3964);
    assert!(id.max_val >= 999_999);
    // Another repaint has other defaults for the same options.
    let blue = setvars_from(&r, &HashMap::new(), "Blue");
    assert_eq!(item(&blue, "display_ext").default_value, 5);
    assert_eq!(item(&blue, "vehicle_id").default_value, 0);
}

#[test]
fn an_empty_repaint_name_means_the_vehicles_default_repaint() {
    let mut r = reps(&[("Standard", "wheels", "1"), ("Other", "wheels", "0")]);
    r.default_name = "Standard".into();
    assert_eq!(
        item(&setvars_from(&r, &HashMap::new(), ""), "wheels").default_value,
        1
    );
    assert_eq!(
        item(&setvars_from(&r, &HashMap::new(), "Other"), "wheels").default_value,
        0
    );
}

#[test]
fn only_declared_options_are_offered_and_decimals_are_left_out() {
    let mut r = reps(&[
        ("A", "vis_x", "1"),
        ("A", "vis_hidden", "1"),
        ("A", "cti_tuergeschw_max", "-0.3"),
    ]);
    r.declared.insert(
        "vis_x".into(),
        BTreeSet::from(["setvar_list.txt".to_string()]),
    );
    r.declared.insert(
        "cti_tuergeschw_max".into(),
        BTreeSet::from(["x.txt".to_string()]),
    );
    let items = setvars_from(&r, &HashMap::new(), "A");
    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["vis_x"],
        "undeclared and decimal options are not offered"
    );
}

#[test]
fn a_documented_family_gives_the_label_and_group() {
    let mut r = reps(&[("Red", "language_ibis", "1")]);
    r.folder = "Citybus 530 by Kajosoft".into();
    let items = setvars_from(&r, &HashMap::new(), "Red");
    let ibis = item(&items, "language_ibis");
    assert_eq!(ibis.group.as_deref(), Some("display"));
    assert_eq!(ibis.kind, SetvarKind::Choice);
    assert!(ibis.values.contains(&0) && ibis.values.contains(&1) && ibis.values.contains(&2));
    // Documented options that the vehicle declares appear although no repaint sets them.
    r.declared.insert("language_ibis".into(), BTreeSet::new());
    r.declared.insert("ramp".into(), BTreeSet::new());
    let with_doc = setvars_from(&r, &HashMap::new(), "Red");
    assert!(with_doc.iter().any(|i| i.name == "ramp"));
}

#[test]
fn stepping_toggles_a_switch_wraps_a_choice_and_clamps_a_number() {
    let make = |kind, values: &[i32], min_val, max_val| SetvarItem {
        name: "x".into(),
        label: "X".into(),
        description: String::new(),
        value: 0,
        default_value: 0,
        min_val,
        max_val,
        kind,
        values: values.to_vec(),
        group: None,
        unverified: false,
    };
    let switch = make(SetvarKind::Switch, &[], 0, 1);
    assert_eq!((switch.step(0, 1), switch.step(1, 1)), (1, 0));
    let choice = make(SetvarKind::Choice, &[0, 2, 5], 0, 5);
    assert_eq!(choice.step(2, 1), 5);
    assert_eq!(choice.step(5, 1), 0, "wraps forward");
    assert_eq!(choice.step(0, -1), 5, "wraps backward");
    assert_eq!(
        choice.step(3, 1),
        2,
        "an unlisted value starts at the first"
    );
    let number = make(SetvarKind::Number, &[], 0, 10);
    assert_eq!(
        (number.step(10, 1), number.step(0, -1), number.step(4, 1)),
        (10, 0, 5)
    );
}
