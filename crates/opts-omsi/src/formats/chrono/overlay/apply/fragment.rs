//! Appending the elements of an added fragment to a tile, with their indices shifted.

use super::*;
use crate::formats::map::RuleOwner;

impl MapTile {
    /// Appends added elements; their rule owners and `[varparent]` links are shifted to the
    /// positions they take here (attachment carriers already count the whole tile).
    pub(super) fn add_fragment(&mut self, frag: &MapTile) {
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
}
