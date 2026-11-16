//! The auditable list of every field the profile can contain, rendered to
//! `docs/src/reference/hardware-fields.md`. The table [`FIELDS`] is checked against the serialised
//! profile by a test, and the page by another (`UPDATE_HARDWARE_DOC=1 cargo test -p opts-hardware
//! reference` rewrites it), so code, charter and page cannot drift apart.

use std::fmt::Write;

use crate::allowlist;
use crate::fact::Reason;
use crate::probe::{GPU_FEATURES, ISA_FEATURES};
use crate::profile::{
    Arch, ChannelLayout, CpuVendor, DisplayServer, Environment, GpuBackend, GpuClass, GpuVendor,
    OsFamily, StorageClass, SCHEMA_VERSION,
};

mod fields;
mod render;

pub use fields::*;
pub use render::*;

#[cfg(test)]
mod tests;
