//! # OPTS OMSI legacy AI lists (`opts-omsi-ailist`)
//!
//! The AI group types (`AiEntry`, `AiGroup`) and the legacy `[ailist]`/`[aigroup]` syntax that
//! fills them (`docs/src/formats/map/lists.md`, "Legacy syntax"). Split out of `opts-omsi-map`
//! (ISS-072) so the legacy reader has its own budget. `opts-omsi-map` re-exports both types, so
//! `formats::map::lists::AiEntry` and `formats::map::lists::AiGroup` keep their paths.
//!
//! Pure in-memory parsing over keyword-block parameters: no file I/O, no engine, no OMSI binaries.

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod legacy;

/// One vehicle of an AI group.
#[derive(Debug, Clone, PartialEq)]
pub struct AiEntry {
    /// Vehicle file as written (`.bus`, `.ovh` or `.zug`; backslashes, relative to the OMSI root).
    pub vehicle: String,
    /// Relative spawn frequency; `None` when the line carries no weight (the default share ❓).
    pub weight: Option<f32>,
}

/// A named set of AI vehicles (`[aigroup_2]` … `[end]`, or a legacy `[aigroup]`).
#[derive(Debug, Clone, PartialEq)]
pub struct AiGroup {
    /// Group name (`NormalCars`, `Trucks`, …); referenced by name from `unsched_*.txt`.
    pub name: String,
    /// Vehicles in file order.
    pub entries: Vec<AiEntry>,
}

impl AiGroup {
    /// Sum of the weights, counting a missing weight as `1`.
    #[must_use]
    pub fn total_weight(&self) -> f32 {
        self.entries.iter().map(|e| e.weight.unwrap_or(1.0)).sum()
    }
}
