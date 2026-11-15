//! Corpus enumeration: every `.sco` below `Sceneryobjects/`, its tier (placed by a map tile,
//! only as a spline attachment, or not at all), selection filters and `objects.jsonl`.

mod entries;
mod jsonl;
mod listing;
mod references;
mod select;

pub use entries::entries;
pub use jsonl::{read_jsonl, write_jsonl};
pub use listing::list_sco_files;
pub use references::map_references;
pub use select::{select, Selection};

#[cfg(test)]
mod tests;
