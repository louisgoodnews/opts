//! Hardware info sections of the profile: OS, CPU, memory, GPU, display, storage, audio, input and benchmarks.

use super::*;

/// `major.minor` of an OS version (digits only; no build strings, no distribution names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionClass {
    /// Major number (Linux kernel, macOS, Windows).
    pub major: u32,
    /// Minor number (0 when unknown).
    pub minor: u32,
}

/// Operating system.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OsInfo {
    /// Family.
    pub family: Fact<OsFamily>,
    /// Version class (kernel on Linux).
    pub version: Fact<VersionClass>,
    /// Windowing system.
    pub display_server: Fact<DisplayServer>,
    /// Bare metal, container or VM.
    pub environment: Fact<Environment>,
}

/// Processor.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CpuInfo {
    /// Architecture class.
    pub arch: Fact<Arch>,
    /// Vendor class.
    pub vendor: Fact<CpuVendor>,
    /// Physical cores.
    pub physical_cores: Fact<u32>,
    /// Logical cores (threads available to the process).
    pub logical_cores: Fact<u32>,
    /// ISA features from the allowlist [`crate::probe::ISA_FEATURES`] that the CPU supports.
    pub features: Fact<Vec<String>>,
    /// L2 cache of one core, KiB.
    pub cache_l2_kib: Fact<u32>,
    /// L3 cache, KiB.
    pub cache_l3_kib: Fact<u32>,
}

/// Memory.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryInfo {
    /// Total RAM (respecting container limits), MiB.
    pub total_mib: Fact<u64>,
    /// Available RAM, MiB.
    pub available_mib: Fact<u64>,
}

/// Graphics adapter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GpuInfo {
    /// Vendor class.
    pub vendor: Fact<GpuVendor>,
    /// Device class.
    pub device_class: Fact<GpuClass>,
    /// Dedicated video memory, MiB (unified-memory systems report the shared pool).
    pub vram_mib: Fact<u64>,
    /// Graphics API.
    pub backend: Fact<GpuBackend>,
    /// Largest 2D texture side.
    pub max_texture_2d: Fact<u32>,
    /// Largest buffer, MiB.
    pub max_buffer_mib: Fact<u64>,
    /// Supported wgpu features from the allowlist [`crate::probe::GPU_FEATURES`].
    pub features: Fact<Vec<String>>,
    /// Hardware ray tracing available.
    pub ray_tracing: Fact<bool>,
    /// A DLSS runtime is loadable (set by the renderer; unknown otherwise).
    pub dlss_runtime: Fact<bool>,
}

/// Primary display.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayInfo {
    /// Number of connected displays.
    pub count: Fact<u32>,
    /// Width of the primary display, pixels.
    pub width: Fact<u32>,
    /// Height of the primary display, pixels.
    pub height: Fact<u32>,
    /// Refresh rate, Hz (rounded).
    pub refresh_hz: Fact<u32>,
    /// Scale factor in percent (100 = 1.0).
    pub scale_percent: Fact<u32>,
    /// HDR output supported.
    pub hdr: Fact<bool>,
}

/// The drive that holds the content.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageInfo {
    /// SSD or HDD.
    pub class: Fact<StorageClass>,
    /// Free space of that volume, GiB.
    pub free_gib: Fact<u64>,
}

/// Audio output.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioInfo {
    /// An output device exists.
    pub device_present: Fact<bool>,
    /// Channel layout of the default output.
    pub channels: Fact<ChannelLayout>,
}

/// Input devices (yes/no and counts; no names).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InputInfo {
    /// A gamepad is connected.
    pub gamepad_present: Fact<bool>,
    /// A steering wheel is connected.
    pub wheel_present: Fact<bool>,
}

/// Optional micro-benchmark results (arbitrary but comparable units).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Benchmarks {
    /// Single-thread integer work, million operations per second.
    pub cpu_single_mops: Fact<u32>,
    /// All-thread integer work, million operations per second.
    pub cpu_multi_mops: Fact<u32>,
    /// Memory copy bandwidth, MiB/s.
    pub memory_mib_s: Fact<u32>,
    /// Sequential read of a scratch file, MiB/s.
    pub storage_read_mib_s: Fact<u32>,
    /// GPU fill rate (needs a render device).
    pub gpu_fill: Fact<u32>,
    /// The hardware-class hash these results were measured on.
    pub measured_for_class: Fact<String>,
}
