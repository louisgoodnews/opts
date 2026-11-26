//! The course's path records and their lookup in the lane graph (ISS-085 stage 1).
//!
//! Moved here from `opts-traffic` so [`LaneGraph::resolve_course`] sits beside the graph it
//! reads. `opts-traffic::course` re-exports every item, so consumer paths are unchanged.

use std::collections::HashMap;

use super::{LaneGraph, LaneId};

/// One path of a course: the element's ID code, the path number and the travel direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CourseLane {
    /// ID code of the spline or object the path belongs to (the lane's `key.0`).
    pub source: u64,
    /// Path number within that element (matched against the lane's `key.1`).
    pub number: u32,
    /// Travelled against the path's own direction.
    pub reversed: bool,
}

/// One course path found in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedLane {
    /// Index into the course's entries (its running order).
    pub course_index: usize,
    /// The graph lane with `Lane.key == (source, number)`.
    pub lane: LaneId,
}

/// The course looked up in a lane graph. Nothing is dropped: entries without a lane are in
/// `unresolved`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    /// The entries that were found, in running order.
    pub resolved: Vec<ResolvedLane>,
    /// `(index in the course, entry)` of every entry no lane has the key of.
    pub unresolved: Vec<(usize, CourseLane)>,
    /// Course indices of entries whose lane is not a successor of the lane before it, where both
    /// lanes resolved and the entries are adjacent in the course (hypothesis H1 of the timetable
    /// page: a course is a chain of successors).
    pub broken_joins: Vec<usize>,
}

impl Resolution {
    /// Share of the course's entries that resolved (`1.0` for an empty course).
    #[must_use]
    pub fn resolved_fraction(&self) -> f64 {
        let total = self.resolved.len() + self.unresolved.len();
        if total == 0 {
            1.0
        } else {
            self.resolved.len() as f64 / total as f64
        }
    }
}

impl LaneGraph {
    /// Looks each entry of `course` up by [`Lane::key`](crate::graph::Lane) and checks the joins
    /// between consecutive entries that were found.
    #[must_use]
    pub fn resolve_course(&self, course: &[CourseLane]) -> Resolution {
        let by_key: HashMap<(u64, u32), LaneId> = self
            .lane_ids()
            .filter_map(|id| self.lane(id).map(|l| (l.key, id)))
            .collect();
        let mut out = Resolution::default();
        for (index, entry) in course.iter().enumerate() {
            match by_key.get(&(entry.source, entry.number)) {
                Some(&lane) => out.resolved.push(ResolvedLane {
                    course_index: index,
                    lane,
                }),
                None => out.unresolved.push((index, *entry)),
            }
        }
        for pair in out.resolved.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b.course_index == a.course_index + 1 && !self.successors(a.lane).contains(&b.lane) {
                out.broken_joins.push(b.course_index);
            }
        }
        out
    }
}
