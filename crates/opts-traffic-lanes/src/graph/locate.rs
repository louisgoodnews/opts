//! Matching a point of the map to lanes: the candidates near a point, the lane the vehicle is on
//! (with hysteresis and a course bonus), and the position along a course.
//!
//! Kind-aware: every query takes an `accept` filter, so a bus matches road lanes, a tram matches
//! rail lanes, and a shared track is matched with both kinds allowed. Design:
//! `docs/src/architecture/minimap-path-recognition.md`, section 4.

use super::*;

/// A lane near a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneCandidate {
    /// The lane.
    pub lane: LaneId,
    /// Distance along the lane of the nearest sample (m).
    pub s_m: f64,
    /// Distance from the point to that sample (m).
    pub lateral_m: f64,
}

impl LaneGraph {
    /// Lanes within `radius` of `point` that `accept` admits, each with its nearest sample (sampled
    /// every metre along the lane; the spatial hash finds the lanes, the samples place them).
    /// Candidates come back in lane-id order.
    #[must_use]
    pub fn lanes_near(
        &self,
        point: DVec2,
        radius: f64,
        accept: impl Fn(&Lane) -> bool,
    ) -> Vec<LaneCandidate> {
        // Lane samples sit every 4 m (see `conflict::index_lanes`), so a lane within `radius` has a
        // sample within `radius + 4` of the point; the cell range covers that square.
        let reach = radius + 4.0;
        let (x0, y0) = crate::conflict::cell(DVec2::new(point.x - reach, point.y - reach));
        let (x1, y1) = crate::conflict::cell(DVec2::new(point.x + reach, point.y + reach));
        let mut ids: Vec<u32> = Vec::new();
        for cx in x0..=x1 {
            for cy in y0..=y1 {
                if let Some(v) = self.grid.get(&(cx, cy)) {
                    ids.extend_from_slice(v);
                }
            }
        }
        ids.sort_unstable();
        ids.dedup();
        let mut out = Vec::new();
        for i in ids {
            let lane = &self.lanes[i as usize];
            if !accept(lane) {
                continue;
            }
            let len = lane.length();
            let mut best: Option<(f64, f64)> = None;
            let mut s = 0.0;
            loop {
                let s_at = s.min(len);
                let d = lane.geometry.point_at(s_at).distance(point);
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((s_at, d));
                }
                if s_at >= len {
                    break;
                }
                s += 1.0;
            }
            if let Some((s_m, lateral_m)) = best {
                if lateral_m <= radius {
                    out.push(LaneCandidate {
                        lane: LaneId(i),
                        s_m,
                        lateral_m,
                    });
                }
            }
        }
        out
    }
}

/// Where the vehicle is along a course (a list of lanes in running order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CourseProgress {
    /// Index into the course of the lane last matched on it.
    pub index: usize,
}

/// The result of placing a matched lane on a course.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CourseState {
    /// The lane is at this index of the course (never before the progress index).
    OnCourse(usize),
    /// The lane is not on the course at or after the progress index.
    OffCourse,
}

impl CourseProgress {
    /// Places `lane` on `course`. The index only moves forward: a lane found at or after the
    /// current index moves it there; a lane behind it, or not on the course, leaves it alone.
    pub fn advance(&mut self, course: &[LaneId], lane: LaneId) -> CourseState {
        match course
            .iter()
            .enumerate()
            .skip(self.index)
            .find(|(_, &l)| l == lane)
        {
            Some((i, _)) => {
                self.index = i;
                CourseState::OnCourse(i)
            }
            None => CourseState::OffCourse,
        }
    }
}

mod matcher;
pub use matcher::{LaneMatch, LaneMatcher, MatchParams};
