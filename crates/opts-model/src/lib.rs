//! `opts-model`: the **native** OPTS content model and its TOML form
//! ([ADR-0014](../../docs/src/adr/0014-data-model-separation.md)).
//!
//! This is what the engine crates (`opts-bevy`, `opts-sim`, `opts-traffic`, `opts-game`) are
//! meant to read. It knows nothing of OMSI: the legacy readers live in `opts-omsi` and the
//! conversion in `opts-importer`; the two never depend on each other, only on `opts-types`.
//!
//! The schema starts small. [`vehicle::VehicleDefinition`] is the first type and a stub that grows
//! with the consumers' migration and with the vehicle profiles of Phase 14
//! ([`source::ContentSource`] is how the engine will ask for content without knowing where it
//! comes from).

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod source;
pub mod vehicle;

pub use source::{ContentError, ContentSource};
pub use vehicle::{AxleDefinition, VehicleDefinition};
