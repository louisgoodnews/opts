//! In-place application of a [`TileOverlay`] onto a base [`MapTile`].

use std::collections::{HashMap, HashSet};

use super::*;
use crate::formats::map::{MapTrafficRule, RuleOwner};

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

    fn edit(
        &mut self,
        target: Target,
        op: &OverlayOp,
        dead: &mut HashSet<Target>,
    ) -> Result<(), String> {
        match op {
            OverlayOp::Rule(e) => {
                let owner = self.rule_owner(target).ok_or_else(|| {
                    format!("[rule] {} on an attached object: skipped", e.rule_type)
                })?;
                let existing = self.rules.iter_mut().find(|r| {
                    r.owner == Some(owner)
                        && r.path_index == e.path_index
                        && r.flag == e.flag
                        && r.rule_type.eq_ignore_ascii_case(&e.rule_type)
                });
                match existing {
                    Some(r) => r.value = e.value,
                    None => self.rules.push(MapTrafficRule {
                        owner: Some(owner),
                        path_index: e.path_index,
                        rule_type: e.rule_type.clone(),
                        value: e.value,
                        flag: e.flag,
                    }),
                }
            }
            OverlayOp::KillRule {
                path_index,
                rule_type,
            } => {
                let owner = self.rule_owner(target).ok_or_else(|| {
                    format!("[kill_rule] {rule_type} on an attached object: skipped")
                })?;
                self.rules.retain(|r| {
                    !(r.owner == Some(owner)
                        && r.path_index == *path_index
                        && r.rule_type.eq_ignore_ascii_case(rule_type))
                });
            }
            OverlayOp::Relabel(strings) => match target {
                Target::Object(i) => self.objects[i].string_vars.clone_from(strings),
                Target::Attached(i) => self.attached_objects[i].string_vars.clone_from(strings),
                Target::SplineAttachment(i) => {
                    return Err(format!(
                        "[relabel] of spline attachment {}: not applied",
                        self.spline_attachments[i].id_code
                    ))
                }
                Target::Spline(i) => {
                    return Err(format!(
                        "[relabel] of spline {}: not applied",
                        self.splines[i].id_code
                    ))
                }
            },
            OverlayOp::Typ(file) => match target {
                Target::Object(i) => self.objects[i].sco_file.clone_from(file),
                Target::Attached(i) => self.attached_objects[i].sco_file.clone_from(file),
                Target::SplineAttachment(i) => self.spline_attachments[i].sco_file.clone_from(file),
                Target::Spline(i) => self.splines[i].sli_file.clone_from(file),
            },
            OverlayOp::Delete => {
                dead.insert(target);
            }
            OverlayOp::SelectObject(_) | OverlayOp::SelectSpline(_) | OverlayOp::Add(_) => {}
        }
        Ok(())
    }

    /// Appends added elements; their rule owners and `[varparent]` links are shifted to the
    /// positions they take here (attachment carriers already count the whole tile).
    fn add_fragment(&mut self, frag: &MapTile) {
        let (o0, s0) = (self.objects.len(), self.splines.len());
        self.objects.extend(frag.objects.iter().cloned());
        self.splines.extend(frag.splines.iter().cloned());
        self.attached_objects
            .extend(frag.attached_objects.iter().cloned());
        self.spline_attachments
            .extend(frag.spline_attachments.iter().cloned());
        self.var_parents
            .extend(frag.var_parents.iter().map(|&(i, p)| (i + o0, p)));
        self.rules.extend(frag.rules.iter().map(|r| {
            let mut r = r.clone();
            r.owner = r.owner.map(|o| match o {
                RuleOwner::Object { index, id_code } => RuleOwner::Object {
                    index: index + o0,
                    id_code,
                },
                RuleOwner::Spline { index, id_code } => RuleOwner::Spline {
                    index: index + s0,
                    id_code,
                },
            });
            r
        }));
    }

    /// Removes deleted elements and re-numbers what refers to objects and splines by position.
    fn remove_dead(&mut self, dead: &HashSet<Target>, warnings: &mut Vec<String>) {
        if dead.is_empty() {
            return;
        }
        let remap = |len: usize, is_dead: &dyn Fn(usize) -> bool| -> HashMap<usize, usize> {
            let mut m = HashMap::new();
            let mut next = 0;
            for i in 0..len {
                if !is_dead(i) {
                    m.insert(i, next);
                    next += 1;
                }
            }
            m
        };
        let objects = remap(self.objects.len(), &|i| dead.contains(&Target::Object(i)));
        let splines = remap(self.splines.len(), &|i| dead.contains(&Target::Spline(i)));
        let n_splines = self.splines.len();
        keep_alive(&mut self.objects, |i| objects.contains_key(&i));
        keep_alive(&mut self.splines, |i| splines.contains_key(&i));
        keep_alive(&mut self.attached_objects, |i| {
            !dead.contains(&Target::Attached(i))
        });
        let mut i = 0;
        self.spline_attachments.retain_mut(|a| {
            let alive = !dead.contains(&Target::SplineAttachment(i));
            i += 1;
            if !alive {
                return false;
            }
            if a.spline_index < n_splines {
                match splines.get(&a.spline_index) {
                    Some(&n) => a.spline_index = n,
                    None => {
                        warnings.push(format!(
                            "attachment {} lost its carrier spline: removed",
                            a.id_code
                        ));
                        return false;
                    }
                }
            }
            true
        });
        self.var_parents
            .retain_mut(|(index, _)| match objects.get(index) {
                Some(&n) => {
                    *index = n;
                    true
                }
                None => false,
            });
        self.rules.retain_mut(|r| match r.owner {
            None => true,
            Some(RuleOwner::Object { index, id_code }) => objects.get(&index).is_some_and(|&n| {
                r.owner = Some(RuleOwner::Object { index: n, id_code });
                true
            }),
            Some(RuleOwner::Spline { index, id_code }) => splines.get(&index).is_some_and(|&n| {
                r.owner = Some(RuleOwner::Spline { index: n, id_code });
                true
            }),
        });
    }
}

fn keep_alive<T>(v: &mut Vec<T>, alive: impl Fn(usize) -> bool) {
    let mut i = 0;
    v.retain(|_| {
        let keep = alive(i);
        i += 1;
        keep
    });
}
