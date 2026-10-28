//! Unit tests of `overlay.rs` (moved out of the file to keep it under the 500-line limit, refactoring cycle 1).

use super::*;
use crate::formats::map::RuleOwner;

fn object(id: u64, sco: &str, strings: &[&str]) -> String {
    let mut s = format!(
        "[object]\n0\n{sco}\n{id}\n10\n20\n0\n90\n0\n0\n{}\n",
        strings.len()
    );
    for v in strings {
        s.push_str(v);
        s.push('\n');
    }
    s.push_str("Object Nr. 1\n\n");
    s
}

fn spline(id: u64, sli: &str) -> String {
    format!("[spline]\n0\n{sli}\n{id}\n0\n0\n0\n0\n0\n0\n50\n0\n0\n0\n0\n0\n0\n0\n\n")
}

fn rule(i: usize, t: &str, v: f32, f: u32) -> String {
    format!("[rule]\n{i}\n{t}\n{v}\n{f}\n\n")
}

fn base() -> MapTile {
    let text = format!(
        "[version]\n14\n\n[terrain]\n\n{}{}{}{}{}{}[attachObj]\n0\nline.sco\n300\n100\n0\n1\n0\n0\n0\n1\n13N\n\n[splineAttachement]\n0\nbollard.sco\n400\n1\n2\n0\n5\n0\n0\n0\n",
        object(100, "a.sco", &["x", "y"]),
        rule(0, "priority", 1.0, 0),
        spline(10, "road.sli"),
        rule(0, "trafficdensity", 1.0, 0),
        rule(1, "speedlimit", 50.0, 0),
        spline(11, "path.sli"),
    );
    MapTile::from_text(&text).unwrap()
}

fn apply(tile: &mut MapTile, overlay: &str) -> OverlayReport {
    tile.apply_overlay(&TileOverlay::from_text(overlay))
}

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

/// Deleting the first spline moves the second down: rule owners and attachment carriers are
/// re-numbered, the deleted spline's rules go.
#[test]
fn delete_renumbers_owners_and_carriers() {
    let mut t = base();
    let r = apply(&mut t, "[selspline]\n10\n[delete]\n");
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert_eq!(t.splines.len(), 1);
    assert_eq!(
        t.spline_attachments[0].spline_index, 0,
        "carrier 11 is now first"
    );
    assert_eq!(t.rules.len(), 1, "only the object's rule is left");
    // Deleting the carrier removes its attachment.
    let mut t = base();
    let r = apply(&mut t, "[selspline]\n11\n[delete]\n");
    assert!(t.spline_attachments.is_empty());
    assert_eq!(r.warnings.len(), 1);
    // Deleting an object.
    let mut t = base();
    apply(
        &mut t,
        "[selobject]\n100\n[delete]\n[selobject]\n300\n[delete]\n",
    );
    assert!(t.objects.is_empty() && t.attached_objects.is_empty());
    assert!(!t
        .rules
        .iter()
        .any(|r| matches!(r.owner, Some(RuleOwner::Object { .. }))));
}

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

#[test]
fn unresolved_selections_skip_their_edits() {
    let mut t = base();
    let before = t.clone();
    let r = apply(&mut t, "[selobject]\n999\n[delete]\n[relabel]\n1\nq\n");
    assert_eq!((r.selections, r.resolved), (1, 0));
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(t, before);
}

/// Overlays never touch terrain or water markers.
#[test]
fn terrain_markers_are_ignored() {
    let mut t = MapTile::default();
    apply(&mut t, "[version]\n14\n[terrain]\n[water]\n");
    assert!(!t.has_terrain && !t.has_water);
}
