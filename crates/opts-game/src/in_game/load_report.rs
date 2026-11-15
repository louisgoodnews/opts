//! What went wrong while loading, kept apart from the status text so it is never overwritten
//! (audit F03): the loading screen ends in *Ready*, *Ready with N problems* or *Failed*
//! (`docs/src/roadmap/phase-9/wizard-and-loading.md` section 7).

use bevy::prelude::*;

/// Start the drive without waiting for *Continue* when the load ends with problems. Set by the
/// control protocol (`loading.continue`, `scene.load_map`) for one load; the command line flag
/// `--accept-load-problems` does the same for every load.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AcceptLoadProblems(pub bool);

/// Start every drive without waiting for *Continue* when its load ends with problems, until
/// switched off: the protocol's form of `--accept-load-problems` for a game already running
/// (`loading.accept_problems {always}`, `scene.load_map {accept_problems: "always"}`). Covers
/// loads started through the plan-a-drive wizard too.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AcceptLoadProblemsAlways(pub bool);

mod problem;
pub use problem::{severity_of, LoadProblem, Severity};
mod report;
pub use report::{LoadOutcome, LoadReport};
mod counts;
pub use counts::{problems_from_counts, WorldCounts};

#[cfg(test)]
mod tests;
