//! File classification and the events of the reload pipeline.

use std::path::{Path, PathBuf};

use bevy::prelude::{Message, Resource};

/// What kind of content a file holds; decides how it is validated and applied.
mod classify;
mod messages;

pub use classify::*;
pub use messages::*;


/// Counters and recent outcomes of the reload pipeline.
#[derive(Resource, Debug, Default)]
pub struct ReloadStats {
    /// Next sequence number.
    pub next_seq: u64,
    /// Requests seen.
    pub requested: u64,
    /// Requests applied.
    pub applied: u64,
    /// Requests that failed validation or application.
    pub failed: u64,
    /// The last [`ReloadStats::KEEP`] outcomes, oldest first.
    pub outcomes: std::collections::VecDeque<ReloadOutcome>,
}

impl ReloadStats {
    /// How many outcomes are kept.
    pub const KEEP: usize = 64;

    /// Allocates a sequence number.
    pub fn allocate(&mut self) -> u64 {
        self.next_seq += 1;
        self.next_seq
    }

    /// Records the end of a request.
    pub fn record(&mut self, outcome: ReloadOutcome) {
        if outcome.result.is_ok() {
            self.applied += 1;
        } else {
            self.failed += 1;
        }
        if self.outcomes.len() == Self::KEEP {
            self.outcomes.pop_front();
        }
        self.outcomes.push_back(outcome);
    }

    /// The outcome of request `seq`, if it has finished.
    #[must_use]
    pub fn outcome(&self, seq: u64) -> Option<&ReloadOutcome> {
        self.outcomes.iter().find(|o| o.seq == seq)
    }
}

/// Present when an application system consumes [`ReloadValidated`] and reports [`ReloadApplied`].
/// Without it the pipeline reports success right after validation.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct ReloadApplierRegistered;

#[cfg(test)]
mod tests;
