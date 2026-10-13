//! The [`HardwareProfile`]: versioned, serialisable, every field a [`Fact`].
//!
//! The list of fields is mirrored in [`crate::reference::FIELDS`]; a test keeps both in step and
//! the generated page `docs/src/reference/hardware-fields.md` follows.

use serde::{Deserialize, Serialize};

use crate::fact::Fact;

/// Version of the serialised schema. Newer files are rejected, older ones load (missing fields
/// become `Unknown(not_probed)`, unknown fields are ignored).
pub const SCHEMA_VERSION: u32 = 1;

macro_rules! class_enum {
    ($(#[$m:meta])* $name:ident { $($(#[$vm:meta])* $v:ident = $k:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($(#[$vm])* $v),+ }
        impl $name {
            /// Stable key (the serialised form).
            #[must_use]
            pub const fn key(self) -> &'static str { match self { $(Self::$v => $k),+ } }
            /// Every value, for the generated reference.
            pub const ALL: &'static [Self] = &[$(Self::$v),+];
        }
    };
}

class_enum!(
    /// Operating-system family.
    OsFamily { Linux = "linux", Macos = "macos", Windows = "windows", Other = "other" }
);
class_enum!(
    /// Windowing system class.
    DisplayServer { Wayland = "wayland", X11 = "x11", Quartz = "quartz", Win32 = "win32", None = "none" }
);
class_enum!(
    /// Whether the program runs on bare metal, in a container or in a virtual machine.
    Environment { Bare = "bare", Container = "container", Vm = "vm" }
);
class_enum!(
    /// CPU architecture class.
    Arch { X86_64 = "x86_64", Aarch64 = "aarch64", Other = "other" }
);
class_enum!(
    /// CPU vendor class (the model string is classified and dropped).
    CpuVendor { Intel = "intel", Amd = "amd", Apple = "apple", Arm = "arm", Other = "other" }
);
class_enum!(
    /// GPU vendor class.
    GpuVendor {
        Nvidia = "nvidia", Amd = "amd", Intel = "intel", Apple = "apple",
        Qualcomm = "qualcomm", Software = "software", Other = "other"
    }
);
class_enum!(
    /// GPU device class.
    GpuClass {
        Discrete = "discrete", Integrated = "integrated", Virtual = "virtual",
        Software = "software", Other = "other"
    }
);
class_enum!(
    /// Graphics API in use.
    GpuBackend {
        Vulkan = "vulkan", Dx12 = "dx12", Metal = "metal", Gl = "gl", Other = "other"
    }
);
class_enum!(
    /// Class of the drive that holds the content.
    StorageClass { Ssd = "ssd", Hdd = "hdd" }
);
class_enum!(
    /// Audio output layout.
    ChannelLayout {
        Mono = "mono", Stereo = "stereo", Surround51 = "surround51",
        Surround71 = "surround71", Other = "other"
    }
);

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

impl HardwareProfile {
    /// Serialises to pretty JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Reads a profile; unknown fields are ignored, missing ones are `Unknown(not_probed)`,
    /// a schema newer than [`SCHEMA_VERSION`] is refused.
    ///
    /// # Errors
    /// [`LoadError`] when the text is not a profile or is too new.
    pub fn from_json(text: &str) -> Result<Self, LoadError> {
        let p: Self = serde_json::from_str(text).map_err(|e| LoadError::Parse(e.to_string()))?;
        if p.schema > SCHEMA_VERSION {
            return Err(LoadError::TooNew(p.schema));
        }
        Ok(p)
    }

    /// Hash of the coarse hardware *class* (FNV-1a over bucketed values). Many machines share a
    /// class; it is used only to decide whether cached benchmarks still apply.
    #[must_use]
    pub fn class_hash(&self) -> String {
        let bucket = |v: Option<u64>, base: u64| {
            v.map_or(0, |x| 64 - (x / base.max(1)).leading_zeros() as u64)
        };
        let parts = [
            format!("{:?}", self.os.family.get()),
            format!("{:?}", self.cpu.arch.get()),
            format!("{:?}", self.cpu.vendor.get()),
            bucket(self.cpu.logical_cores.get().map(u64::from), 1).to_string(),
            bucket(self.memory.total_mib.get(), 1024).to_string(),
            format!("{:?}", self.gpu.vendor.get()),
            format!("{:?}", self.gpu.device_class.get()),
            bucket(self.gpu.vram_mib.get(), 1024).to_string(),
            format!("{:?}", self.storage.class.get()),
        ];
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in parts.join("|").bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        format!("{h:016x}")
    }
}

macro_rules! all_unknown {
    ($ty:ident { $($f:ident),+ }) => {
        impl $ty {
            /// Every fact unknown for one `reason`.
            #[must_use]
            pub const fn unknown(reason: crate::fact::Reason) -> Self {
                Self { $($f: Fact::Unknown(reason)),+ }
            }
        }
    };
}

all_unknown!(OsInfo {
    family,
    version,
    display_server,
    environment
});
all_unknown!(CpuInfo {
    arch,
    vendor,
    physical_cores,
    logical_cores,
    features,
    cache_l2_kib,
    cache_l3_kib
});
all_unknown!(MemoryInfo {
    total_mib,
    available_mib
});
all_unknown!(GpuInfo {
    vendor,
    device_class,
    vram_mib,
    backend,
    max_texture_2d,
    max_buffer_mib,
    features,
    ray_tracing,
    dlss_runtime
});
all_unknown!(DisplayInfo {
    count,
    width,
    height,
    refresh_hz,
    scale_percent,
    hdr
});
all_unknown!(StorageInfo { class, free_gib });
all_unknown!(AudioInfo {
    device_present,
    channels
});
all_unknown!(InputInfo {
    gamepad_present,
    wheel_present
});
all_unknown!(Benchmarks {
    cpu_single_mops,
    cpu_multi_mops,
    memory_mib_s,
    storage_read_mib_s,
    gpu_fill,
    measured_for_class
});
