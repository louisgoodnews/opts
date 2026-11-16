//! The selection persisting over several edits, and the elements it adds.

use super::fixtures::*;
use crate::formats::map::RuleOwner;

/// The selection persists over several edits and is cleared by an added element: the rule
/// after it belongs to the new spline. Added attachments count carriers over the whole tile.
#[test]
fn selection_persists_and_added_elements_take_their_rules() {
    let mut t = base();
    let overlay = format!(
        "[version]\n14\n[selobject]\n100\n{}[relabel]\n1\nz\n{}{}[splineAttachement]\n0\nsign.sco\n500\n2\n1\n0\n3\n0\n0\n0\n{}[selspline]\n12\n{}",
        rule(0, "priority", 2.0, 0),
        spline(12, "new.sli"),
        rule(0, "speedlimit", 30.0, 0),
        object(101, "b.sco", &[]),
        rule(1, "trafficdensity", 0.0, 0),
    );
    let r = apply(&mut t, &overlay);
    assert_eq!(
        (r.selections, r.resolved),
        (2, 2),
        "the second selection finds the added spline"
    );
    assert_eq!(t.objects[0].string_vars, vec!["z"]);
    assert_eq!(t.splines.len(), 3);
    assert_eq!(t.objects.len(), 2);
    let on12: Vec<_> = t
        .rules
        .iter()
        .filter(|r| {
            r.owner
                == Some(RuleOwner::Spline {
                    index: 2,
                    id_code: 12,
                })
        })
        .map(|r| r.rule_type.as_str())
        .collect();
    assert_eq!(on12, vec!["speedlimit", "trafficdensity"]);
    assert_eq!(t.spline_attachments[1].spline_index, 2);
    assert!(t
        .rules
        .iter()
        .any(|r| r.rule_type == "priority" && r.value == 2.0));
}
