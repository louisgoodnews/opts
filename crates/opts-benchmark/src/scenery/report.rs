//! Report files: `results.jsonl` (appended and flushed per object), `summary.csv`,
//! `categories.json`, `report.md`, and the comparison of two runs.

mod category;
mod compare;
mod markdown;
mod results;
mod summary;

pub use category::categories;
pub use compare::compare;
pub use results::{latest, read_results, ResultsWriter};
pub use summary::write_summaries;

#[cfg(test)]
mod tests;
