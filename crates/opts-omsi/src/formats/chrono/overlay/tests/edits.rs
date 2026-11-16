//! Deletions, unresolved selections and terrain markers of an overlay.

use super::super::*;
use super::fixtures::*;
use crate::formats::map::RuleOwner;

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
