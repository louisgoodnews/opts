//! Removing the deleted elements of a tile and re-numbering what refers to them by position.

use std::collections::{HashMap, HashSet};

use super::Target;
use super::*;
use crate::formats::map::RuleOwner;

impl MapTile {
    /// Removes deleted elements and re-numbers what refers to objects and splines by position.
    pub(super) fn remove_dead(&mut self, dead: &HashSet<Target>, warnings: &mut Vec<String>) {
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
