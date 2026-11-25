//! Data structures for scene trace records, mesh notes, probes, bounds, and transforms.

mod geometry;
pub mod kind;
mod lap;
mod mesh;
mod query;
mod record;

pub use geometry::{union_bounds, TraceBounds, TraceTransform};
pub use lap::Lap;
pub use mesh::{MaterialNote, MeshNote};
pub use query::{Probe, Query};
pub use record::{ParentRef, TraceRecord};

/// Build information written into the header record.
#[must_use]
pub fn build_info() -> serde_json::Value {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
    })
}
