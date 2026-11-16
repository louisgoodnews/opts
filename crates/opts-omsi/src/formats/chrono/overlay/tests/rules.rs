//! Rule upserts, rule kills, relabels and type replacements of an overlay.

use super::fixtures::*;
use crate::formats::map::RuleOwner;

#[test]
fn rule_upserts_by_owner_path_type_and_flag() {
    let mut t = base();
    let r = apply(
        &mut t,
        &format!(
            "[selspline]\n10\n\n{}{}",
            rule(0, "TrafficDensity", 4.0, 0),
            rule(0, "trafficdensity", 2.0, 3)
        ),
    );
    assert_eq!((r.selections, r.resolved), (1, 1));
    let on10: Vec<_> = t
        .rules
        .iter()
        .filter(|r| matches!(r.owner, Some(RuleOwner::Spline { id_code: 10, .. })))
        .collect();
    assert_eq!(on10.len(), 3, "updated one, added one with another flag");
    assert_eq!(on10[0].value, 4.0);
    assert_eq!((on10[2].value, on10[2].flag), (2.0, 3));
}

#[test]
fn kill_rule_removes_by_path_and_type() {
    let mut t = base();
    apply(
        &mut t,
        "[selspline]\n10\n[kill_rule]\n1\nspeedlimit\n0\n7\n",
    );
    assert!(!t.rules.iter().any(|r| r.rule_type == "speedlimit"));
    assert_eq!(t.rules.len(), 2);
}

/// `[relabel]` takes exactly `n` lines, blank ones included, and replaces all strings.
#[test]
fn relabel_keeps_blank_values() {
    let mut t = base();
    apply(&mut t, "[selobject]\n100\n[relabel]\n3\nfirst\n\nthird\nObject Nr. 5\n\n[selobject]\n300\n[relabel]\n1\n\n");
    assert_eq!(t.objects[0].string_vars, vec!["first", "", "third"]);
    assert_eq!(t.attached_objects[0].string_vars, vec![""]);
}

#[test]
fn typ_replaces_sco_and_sli() {
    let mut t = base();
    apply(&mut t, "[selobject]\n300\n[typ]\nnew_line.sco\n[selobject]\n400\n[typ]\nnew_bollard.sco\n[selspline]\n11\n[typ]\nnew.sli\n");
    assert_eq!(t.attached_objects[0].sco_file, "new_line.sco");
    assert_eq!(t.spline_attachments[0].sco_file, "new_bollard.sco");
    assert_eq!(t.splines[1].sli_file, "new.sli");
}
