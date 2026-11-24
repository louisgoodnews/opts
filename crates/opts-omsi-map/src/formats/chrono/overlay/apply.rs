//! In-place application of a [`TileOverlay`] onto a base [`MapTile`].

use std::collections::HashSet;

use super::*;
use crate::formats::map::RuleOwner;

mod edit;
mod fragment;
mod remove;

/// The element a selection found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Target {
    Object(usize),
    Attached(usize),
    SplineAttachment(usize),
    Spline(usize),
}

impl MapTile {
    /// Applies one overlay ([`TileOverlay`]) in place.
    ///
    /// Deleted elements are removed after the whole overlay (indices inside one overlay refer to
    /// the tile as it was before it plus the elements it added); rule owners, `[varparent]` links
    /// and attachment carriers are then re-numbered. Selections that find nothing skip their
    /// edits with a warning.
    pub fn apply_overlay(&mut self, overlay: &TileOverlay) -> OverlayReport {
        let mut report = OverlayReport::default();
        let mut dead: HashSet<Target> = HashSet::new();
        // `None`: no selection; `Some(None)`: a selection that found nothing.
        let mut sel: Option<Option<Target>> = None;
        for op in &overlay.ops {
            match op {
                OverlayOp::SelectObject(id) => {
                    report.selections += 1;
                    let alive = |t: Target| !dead.contains(&t);
                    let found = self
                        .objects
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| o.id_code == *id)
                        .map(|(i, _)| Target::Object(i))
                        .find(|t| alive(*t))
                        .or_else(|| {
                            self.attached_objects
                                .iter()
                                .enumerate()
                                .filter(|(_, o)| o.id_code == *id)
                                .map(|(i, _)| Target::Attached(i))
                                .find(|t| alive(*t))
                        })
                        .or_else(|| {
                            self.spline_attachments
                                .iter()
                                .enumerate()
                                .filter(|(_, o)| o.id_code == *id)
                                .map(|(i, _)| Target::SplineAttachment(i))
                                .find(|t| alive(*t))
                        });
                    if found.is_some() {
                        report.resolved += 1;
                    } else {
                        report.warnings.push(format!(
                            "[selobject] {id}: no such element; its edits are skipped"
                        ));
                    }
                    sel = Some(found);
                }
                OverlayOp::SelectSpline(id) => {
                    report.selections += 1;
                    let found = self
                        .splines
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| s.id_code == *id)
                        .map(|(i, _)| Target::Spline(i))
                        .find(|t| !dead.contains(t));
                    if found.is_some() {
                        report.resolved += 1;
                    } else {
                        report.warnings.push(format!(
                            "[selspline] {id}: no such spline; its edits are skipped"
                        ));
                    }
                    sel = Some(found);
                }
                OverlayOp::Add(frag) => {
                    sel = None;
                    self.add_fragment(frag);
                    report.warnings.extend(frag.warnings.iter().cloned());
                }
                edit => {
                    let Some(Some(target)) = sel else {
                        continue;
                    };
                    if let Err(w) = self.edit(target, edit, &mut dead) {
                        report.warnings.push(w);
                    }
                }
            }
        }
        self.remove_dead(&dead, &mut report.warnings);
        report
    }

    fn rule_owner(&self, target: Target) -> Option<RuleOwner> {
        match target {
            Target::Object(index) => Some(RuleOwner::Object {
                index,
                id_code: self.objects[index].id_code,
            }),
            Target::Spline(index) => Some(RuleOwner::Spline {
                index,
                id_code: self.splines[index].id_code,
            }),
            _ => None,
        }
    }
}
