//! Feed planning, per-frame input injection, and script output table definitions.

use super::*;

#[derive(Debug, Clone, Copy)]
enum Slot {
    Scalar(usize),
    Wheel(WheelFamily, usize, bool),
    Door(usize, usize),
}

#[derive(Debug, Clone)]
struct Planned {
    name: String,
    slot: Slot,
    kind: Kind,
    id: u32,
    /// The set stores to the name: the host leaves it alone.
    script_owned: bool,
    live: bool,
}

/// The feed of one engine, resolved once: variable ids and the names the scripts own.
#[derive(Debug, Clone)]
pub struct FeedPlan {
    rows: Vec<Planned>,
}

mod build;
mod output;
mod query;

pub use output::{output_row, Consumer, OutputRow, OUTPUT_TABLE};
pub use query::feed;
