//! The scorer: applies run events to a challenge and produces its result.

use tracing::{debug, info};

use crate::challenge::Challenge;
use crate::event::ChallengeEvent;
use crate::outcome::{ChallengeResult, Outcome};

/// Running score and tallies of one challenge run.
#[derive(Debug, Clone, PartialEq)]
pub struct Scorer {
    challenge: Challenge,
    score: i64,
    on_time: u32,
    off_time: u32,
    missed: u32,
    collisions: u32,
    passengers: u32,
}

impl Scorer {
    /// A scorer for a fresh run of `challenge`.
    pub fn new(challenge: Challenge) -> Self {
        Self { challenge, score: 0, on_time: 0, off_time: 0, missed: 0, collisions: 0, passengers: 0 }
    }

    /// The score so far.
    pub fn score(&self) -> i64 {
        self.score
    }

    /// Applies one event and returns its score delta.
    pub fn apply(&mut self, event: ChallengeEvent) -> i64 {
        let delta = self.tally(event);
        self.score += delta;
        debug!(
            challenge = self.challenge.id,
            event = ?event,
            delta,
            score = self.score,
            "challenge event scored"
        );
        delta
    }

    /// Updates the counters for `event` and returns the points it is worth.
    fn tally(&mut self, event: ChallengeEvent) -> i64 {
        let rules = self.challenge.rules;
        match event {
            ChallengeEvent::StopServed { scheduled_s, actual_s, .. } => {
                let gap = (actual_s - scheduled_s).abs();
                if gap <= rules.punctual_window_s {
                    self.on_time += 1;
                    rules.on_time_points
                } else {
                    self.off_time += 1;
                    let off_s = (gap - rules.punctual_window_s).ceil() as i64;
                    (rules.off_time_points - rules.points_lost_per_off_s * off_s).max(0)
                }
            }
            ChallengeEvent::StopMissed { .. } => {
                self.missed += 1;
                rules.missed_penalty
            }
            ChallengeEvent::Collision => {
                self.collisions += 1;
                rules.collision_penalty
            }
            ChallengeEvent::PassengersBoarded { count } => {
                self.passengers += count;
                rules.points_per_passenger * i64::from(count)
            }
        }
    }

    /// Pass or fail against the challenge's criteria. A challenge with no expected stops fails.
    fn outcome(&self) -> Outcome {
        let criteria = self.challenge.criteria;
        let ratio = if self.challenge.expected_stops == 0 {
            0.0
        } else {
            f64::from(self.on_time) / f64::from(self.challenge.expected_stops)
        };
        let passed = ratio >= criteria.min_on_time_ratio
            && self.collisions <= criteria.max_collisions
            && self.missed <= criteria.max_missed;
        if passed {
            Outcome::Passed
        } else {
            Outcome::Failed
        }
    }

    /// Ends the run and returns its result. Logs the completed challenge at `info`.
    pub fn finish(&self) -> ChallengeResult {
        let outcome = self.outcome();
        info!(
            challenge = self.challenge.id,
            score = self.score,
            outcome = ?outcome,
            on_time = self.on_time,
            missed = self.missed,
            collisions = self.collisions,
            "challenge completed"
        );
        ChallengeResult {
            challenge: self.challenge.id,
            score: self.score,
            outcome,
            on_time: self.on_time,
            off_time: self.off_time,
            missed: self.missed,
            collisions: self.collisions,
            passengers: self.passengers,
        }
    }
}
