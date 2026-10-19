//! Per-map configuration profiles and the load profiler
//! (`docs/src/architecture/map-profiles.md`, [M-0130](../../../../docs/src/roadmap/phase-9/M-0130.md)).
//!
//! * [`profile`]: the profile, its key (map and Chrono entry), presets and TOML text.
//! * [`store`]: the files under `<data dir>/profiles/`, the fallback chain, the metrics file.
//! * [`advice`]: [`advice::MapLoadProfile`] and the heuristic engine.

pub mod advice;
pub mod profile;
pub mod store;

pub use advice::{advise, Advice, AdviceId, MapLoadProfile, Severity};
pub use profile::{presets, slug, MapKey, MapProfile, DEFAULT_CHRONO};
pub use store::{ProfileSource, ProfileStore, Resolved, SessionMetrics};

#[cfg(test)]
mod tests;
