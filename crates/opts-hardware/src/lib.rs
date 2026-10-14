//! Privacy-respecting hardware capability probe and default recommendations.
//!
//! Charter: `docs/src/architecture/hardware-probe.md` (ADR-0010). Collects only capability
//! facts, never identifiers, contains no network code, works on Linux (any distribution),
//! macOS and Windows with one function surface per platform.
#![forbid(unsafe_code)]

pub mod allowlist;
pub mod bench;
pub mod fact;
pub mod inputs;
pub mod parse;
pub mod platform;
pub mod probe;
pub mod profile;
pub mod recommend;
pub mod reference;
pub mod report;
pub mod store;

pub use fact::{Fact, Reason};
pub use inputs::{AudioInput, DisplayInput, GpuInput, InputDevicesInput, ProbeInputs};
pub use probe::{probe, probe_with};
pub use profile::{HardwareProfile, SCHEMA_VERSION};
