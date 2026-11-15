//! Editing one selected element of a tile: rules, the fields of a selection and deletes.

use std::collections::HashSet;

use super::Target;
use super::*;
use crate::formats::map::MapTrafficRule;

impl MapTile {
    pub(super) fn edit(
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
}
