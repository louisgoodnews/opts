//! The lane matcher: candidate scoring with heading and course bonuses, and the hysteresis that
//! keeps a match from flickering between two lanes.

use super::*;

/// Tuning of the matcher. The defaults are the design's first values, not calibrated against OMSI.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchParams {
    /// Candidates within this distance (m).
    pub radius_m: f64,
    /// Lanes whose heading differs more than this (radians) are dropped.
    pub max_heading_diff: f64,
    /// Score penalty per radian of heading difference (m / rad).
    pub heading_weight_m: f64,
    /// Score bonus for a lane on the course (m).
    pub course_bonus_m: f64,
    /// Score bonus for a successor of the current lane (m).
    pub successor_bonus_m: f64,
    /// A challenger must beat the current lane by this much (m) ...
    pub switch_margin_m: f64,
    /// ... on this many consecutive updates before the match moves.
    pub switch_updates: u8,
}

impl Default for MatchParams {
    fn default() -> Self {
        Self {
            radius_m: 15.0,
            max_heading_diff: 60f64.to_radians(),
            heading_weight_m: 3.0,
            course_bonus_m: 3.0,
            successor_bonus_m: 2.0,
            switch_margin_m: 1.5,
            switch_updates: 2,
        }
    }
}

/// The lane the vehicle is on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaneMatch {
    /// The matched lane.
    pub lane: LaneId,
    /// Distance along it (m).
    pub s_m: f64,
    /// Distance from the vehicle to the lane (m).
    pub lateral_m: f64,
}

#[derive(Debug, Clone, Copy)]
struct Scored {
    cand: LaneCandidate,
    score: f64,
}

/// Keeps the match from one update to the next (the hysteresis state).
#[derive(Debug, Clone, Default)]
pub struct LaneMatcher {
    params: MatchParams,
    current: Option<LaneId>,
    challenger: Option<(LaneId, u8)>,
}

impl LaneMatcher {
    /// A matcher with the given tuning.
    #[must_use]
    pub fn new(params: MatchParams) -> Self {
        Self {
            params,
            current: None,
            challenger: None,
        }
    }

    /// The matched lane, if any.
    #[must_use]
    pub fn current(&self) -> Option<LaneId> {
        self.current
    }

    /// Forgets the match (the vehicle was respawned or the map changed).
    pub fn reset(&mut self) {
        self.current = None;
        self.challenger = None;
    }

    /// One update at `point` facing `heading` (radians, the geometry's own convention). `course`
    /// are the lanes of the active course (a bonus, not a constraint); `accept` filters by kind.
    /// Returns the match, or `None` when no admitted lane lies in reach.
    pub fn update(
        &mut self,
        graph: &LaneGraph,
        point: DVec2,
        heading: f64,
        course: &[LaneId],
        accept: impl Fn(&Lane) -> bool,
    ) -> Option<LaneMatch> {
        let p = self.params;
        let mut scored: Vec<Scored> = Vec::new();
        for cand in graph.lanes_near(point, p.radius_m, accept) {
            let Some(lane) = graph.lane(cand.lane) else {
                continue;
            };
            let diff = heading_gap(lane.geometry.heading_at(cand.s_m), heading);
            if diff > p.max_heading_diff {
                continue;
            }
            let mut score = cand.lateral_m + p.heading_weight_m * diff;
            if course.contains(&cand.lane) {
                score -= p.course_bonus_m;
            }
            if let Some(cur) = self.current {
                if graph.successors(cur).contains(&cand.lane) {
                    score -= p.successor_bonus_m;
                }
            }
            scored.push(Scored { cand, score });
        }
        let best = scored
            .iter()
            .copied()
            .min_by(|a, b| a.score.total_cmp(&b.score).then(a.cand.lane.cmp(&b.cand.lane)));
        let current_scored = self
            .current
            .and_then(|cur| scored.iter().copied().find(|s| s.cand.lane == cur));
        let Some(best) = best else {
            self.reset();
            return None;
        };
        match current_scored {
            // Current lane still a candidate: stay unless a challenger has been clearly better
            // for enough consecutive updates.
            Some(cur) => {
                if best.cand.lane == cur.cand.lane || best.score + p.switch_margin_m >= cur.score {
                    self.challenger = None;
                    self.current = Some(cur.cand.lane);
                    return Some(to_match(cur.cand));
                }
                let count = match self.challenger {
                    Some((lane, n)) if lane == best.cand.lane => n.saturating_add(1),
                    _ => 1,
                };
                if count >= p.switch_updates {
                    self.switch_to(best);
                } else {
                    self.challenger = Some((best.cand.lane, count));
                    self.current = Some(cur.cand.lane);
                    return Some(to_match(cur.cand));
                }
            }
            // No current lane, or it left reach: take the best candidate at once.
            None => self.switch_to(best),
        }
        Some(to_match(best.cand))
    }

    fn switch_to(&mut self, best: Scored) {
        self.current = Some(best.cand.lane);
        self.challenger = None;
    }
}

fn to_match(c: LaneCandidate) -> LaneMatch {
    LaneMatch {
        lane: c.lane,
        s_m: c.s_m,
        lateral_m: c.lateral_m,
    }
}

/// The smaller angle between two headings (radians, `0..=PI`).
fn heading_gap(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(std::f64::consts::TAU);
    d.min(std::f64::consts::TAU - d)
}
