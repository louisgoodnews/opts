//! The result of a finished run, and the hook a career system (ISS-047) implements to store it.

/// Whether the run met its pass criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every criterion was met.
    Passed,
    /// At least one criterion was missed.
    Failed,
}

/// What a finished run produced: the score, the tallies and the outcome.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChallengeResult {
    /// The challenge id.
    pub challenge: &'static str,
    /// Total points (may be negative).
    pub score: i64,
    /// Pass or fail.
    pub outcome: Outcome,
    /// Stops served on time.
    pub on_time: u32,
    /// Stops served off schedule.
    pub off_time: u32,
    /// Stops missed.
    pub missed: u32,
    /// Collisions.
    pub collisions: u32,
    /// Passengers boarded.
    pub passengers: u32,
}

/// Where a finished result goes. The career system implements this; the challenge crate does not
/// depend on it.
pub trait ResultSink {
    /// Store one finished result.
    fn record(&mut self, result: &ChallengeResult);
}

impl ResultSink for Vec<ChallengeResult> {
    fn record(&mut self, result: &ChallengeResult) {
        self.push(*result);
    }
}
