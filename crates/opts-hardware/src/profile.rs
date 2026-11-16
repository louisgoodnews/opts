//! The [`HardwareProfile`]: versioned, serialisable, every field a [`Fact`].
//!
//! The list of fields is mirrored in [`crate::reference::FIELDS`]; a test keeps both in step and
//! the generated page `docs/src/reference/hardware-fields.md` follows.

use serde::{Deserialize, Serialize};

use crate::fact::Fact;

mod classes;
mod infos;
mod methods;
mod unknown;

pub use classes::*;
pub use infos::*;

/// Version of the serialised schema. Newer files are rejected, older ones load (missing fields
/// become `Unknown(not_probed)`, unknown fields are ignored).
pub const SCHEMA_VERSION: u32 = 1;













/// Everything the probe knows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HardwareProfile {
    /// Schema version, [`SCHEMA_VERSION`].
    pub schema: u32,
    /// Operating system.
    pub os: OsInfo,
    /// Processor.
    pub cpu: CpuInfo,
    /// Memory.
    pub memory: MemoryInfo,
    /// Graphics adapter.
    pub gpu: GpuInfo,
    /// Primary display.
    pub display: DisplayInfo,
    /// Content drive.
    pub storage: StorageInfo,
    /// Audio output.
    pub audio: AudioInfo,
    /// Input devices.
    pub input: InputInfo,
    /// Optional benchmark results.
    pub bench: Benchmarks,
    /// How long the probe took, milliseconds.
    pub probe_ms: u32,
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self {
            schema: SCHEMA_VERSION,
            os: OsInfo::default(),
            cpu: CpuInfo::default(),
            memory: MemoryInfo::default(),
            gpu: GpuInfo::default(),
            display: DisplayInfo::default(),
            storage: StorageInfo::default(),
            audio: AudioInfo::default(),
            input: InputInfo::default(),
            bench: Benchmarks::default(),
            probe_ms: 0,
        }
    }
}

/// Why a profile could not be read.
#[derive(Debug)]
pub enum LoadError {
    /// Not valid JSON for the schema.
    Parse(String),
    /// Written by a newer program.
    TooNew(u32),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "hardware profile unreadable: {e}"),
            Self::TooNew(v) => write!(
                f,
                "hardware profile schema {v} is newer than {SCHEMA_VERSION}"
            ),
        }
    }
}

impl std::error::Error for LoadError {}



