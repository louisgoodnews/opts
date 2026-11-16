//! Unit tests for preflight.

use super::*;

#[test]
fn string_sets_differ_and_are_never_empty() {
    let plan = StringPlan::Text(3);
    let (a, b) = (plan.strings('A'), plan.strings('B'));
    assert_eq!(a, vec!["OPTS-A1", "OPTS-A2", "OPTS-A3"]);
    assert_eq!(b.len(), 3);
    assert!(a.iter().zip(&b).all(|(x, y)| x != y && !x.is_empty()));
}

#[test]
fn tree_plan_writes_texture_height_ratio() {
    let plan = StringPlan::Tree {
        texture: "tree.tga".into(),
        height: 12.5,
        ratio: 0.75,
    };
    assert_eq!(plan.strings('A'), vec!["tree.tga", "12.5", "0.75"]);
}
