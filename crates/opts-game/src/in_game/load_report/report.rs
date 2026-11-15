//! The report of a load: how it ended, the problems and what the user decided about them.

use bevy::prelude::Resource;

use super::*;

/// How the loading ended (or that it has not yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoadOutcome {
    /// Still working.
    #[default]
    Loading,
    /// Done without problems: the screen continues by itself.
    Ready,
    /// Usable, but with problems: the screen stays until the user continues.
    ReadyWithProblems,
    /// Nothing usable: the screen stays and offers retry, another choice and back.
    Failed,
}

/// The problems of the current load and what the user decided about them.
#[derive(Resource, Debug, Clone, Default)]
pub struct LoadReport {
    /// Everything that went wrong, in the order it happened.
    pub problems: Vec<LoadProblem>,
    /// The outcome once the loader has finished.
    pub outcome: LoadOutcome,
    /// The user pressed *Continue* on *Ready with problems*.
    pub continue_requested: bool,
    /// The details view is open.
    pub details_open: bool,
    /// The announcement of minor problems after the first HUD frame was made.
    pub minor_announced: bool,
    /// The user pressed *Retry*; the pipeline starts over at its next step.
    pub retry_requested: bool,
}

impl LoadReport {
    /// Records a problem (never replaced by later status text).
    pub fn add(&mut self, problem: LoadProblem) {
        tracing::warn!(problem = problem.key, detail = %problem.detail, "load problem");
        self.problems.push(problem);
    }

    /// Problems that stop the screen.
    pub fn blocking_count(&self) -> usize {
        self.problems
            .iter()
            .filter(|p| p.severity() == Severity::Blocking)
            .count()
    }

    /// Problems that only get announced after the drive has started.
    pub fn minor_count(&self) -> usize {
        self.count() - self.blocking_count()
    }

    /// Whether a fatal problem was recorded.
    pub fn has_fatal(&self) -> bool {
        self.problems.iter().any(|p| p.fatal)
    }

    /// Number of problems.
    pub fn count(&self) -> usize {
        self.problems.len()
    }

    /// Settles the outcome when the loader has finished: fatal problems fail, any other problem
    /// is *Ready with problems*, none is *Ready*.
    pub fn finish(&mut self) {
        self.outcome = if self.has_fatal() {
            LoadOutcome::Failed
        } else if self.blocking_count() == 0 {
            LoadOutcome::Ready
        } else {
            LoadOutcome::ReadyWithProblems
        };
    }

    /// Forgets everything (retry).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The report as plain text for *Copy* and for bug reports.
    pub fn as_text(&self) -> String {
        let mut out = String::new();
        for (i, p) in self.problems.iter().enumerate() {
            out.push_str(&format!(
                "{}. {} {:?}{}\n   {}\n",
                i + 1,
                p.key,
                p.args,
                if p.fatal { " (fatal)" } else { "" },
                p.detail.replace('\n', "\n   ")
            ));
        }
        out
    }
}
