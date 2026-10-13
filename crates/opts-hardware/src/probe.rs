//! The probe: one call fills a [`HardwareProfile`] within a time budget.
//!
//! Platform differences live behind [`PlatformProbe`]; memory, CPU and disks come from `sysinfo`
//! on every platform. A stage that starts after the deadline is skipped and its facts are
//! `Unknown(timeout)`.

use std::path::Path;
use std::time::Instant;

use sysinfo::{CpuRefreshKind, DiskKind, Disks, MemoryRefreshKind, System};

use crate::fact::{Fact, Reason};
use crate::inputs::{AudioInput, DisplayInput, GpuInput, InputDevicesInput, ProbeInputs};
use crate::parse;
use crate::platform::{Native, PlatformProbe};
use crate::profile::{
    AudioInfo, Benchmarks, ChannelLayout, CpuInfo, DisplayInfo, GpuInfo, HardwareProfile,
    InputInfo, MemoryInfo, OsFamily, OsInfo, StorageClass, StorageInfo, SCHEMA_VERSION,
};

/// Every ISA feature name the probe may report (x86_64 and aarch64).
pub const ISA_FEATURES: &[&str] = &[
    "sse4.2", "avx", "avx2", "fma", "bmi2", "popcnt", "avx512f", "neon", "aes", "crc", "dotprod",
];

/// Every wgpu feature name the probe may keep from the renderer's list.
pub const GPU_FEATURES: &[&str] = &[
    "texture_compression_bc",
    "texture_compression_etc2",
    "texture_compression_astc",
    "indirect_first_instance",
    "multi_draw_indirect",
    "timestamp_query",
    "shader_f16",
    "texture_binding_array",
    "float32_filterable",
    "ray_query",
];

struct Clock {
    start: Instant,
    budget: std::time::Duration,
}

impl Clock {
    fn left(&self) -> bool {
        self.start.elapsed() < self.budget
    }
}

fn os_family() -> OsFamily {
    match std::env::consts::OS {
        "linux" => OsFamily::Linux,
        "macos" => OsFamily::Macos,
        "windows" => OsFamily::Windows,
        _ => OsFamily::Other,
    }
}

fn probe_os(p: &dyn PlatformProbe) -> OsInfo {
    // On Linux the kernel version is the class; elsewhere the OS version.
    let text = if cfg!(target_os = "linux") {
        System::kernel_version()
    } else {
        System::os_version()
    };
    OsInfo {
        family: Fact::Known(os_family()),
        version: Fact::from_option(
            text.as_deref().and_then(parse::version_class),
            Reason::Unavailable,
        ),
        display_server: p.display_server(),
        environment: p.environment(),
    }
}

fn isa_features() -> Vec<String> {
    let mut v: Vec<&str> = Vec::new();
    #[cfg(target_arch = "x86_64")]
    {
        macro_rules! x86 {
            ($($f:tt => $n:literal),*) => { $(if std::arch::is_x86_feature_detected!($f) { v.push($n); })* };
        }
        x86!("sse4.2" => "sse4.2", "avx" => "avx", "avx2" => "avx2", "fma" => "fma",
             "bmi2" => "bmi2", "popcnt" => "popcnt", "avx512f" => "avx512f");
    }
    #[cfg(target_arch = "aarch64")]
    {
        macro_rules! arm {
            ($($f:tt => $n:literal),*) => { $(if std::arch::is_aarch64_feature_detected!($f) { v.push($n); })* };
        }
        arm!("neon" => "neon", "aes" => "aes", "crc" => "crc", "dotprod" => "dotprod");
    }
    v.into_iter().map(str::to_owned).collect()
}

fn probe_cpu(p: &dyn PlatformProbe) -> CpuInfo {
    let mut sys = System::new();
    sys.refresh_cpu_list(CpuRefreshKind::nothing());
    let first = sys.cpus().first();
    let (l2, l3) = p.cpu_caches_kib();
    let logical = std::thread::available_parallelism()
        .ok()
        .map(|n| n.get())
        .or_else(|| Some(sys.cpus().len()).filter(|n| *n > 0));
    let to_u32 = |n: usize| u32::try_from(n).ok();
    CpuInfo {
        arch: Fact::Known(parse::arch(std::env::consts::ARCH)),
        vendor: Fact::from_option(
            first.map(|c| parse::cpu_vendor(c.vendor_id(), c.brand())),
            Reason::Unavailable,
        ),
        physical_cores: Fact::from_option(
            System::physical_core_count().and_then(to_u32),
            Reason::Unavailable,
        ),
        logical_cores: Fact::from_option(logical.and_then(to_u32), Reason::Unavailable),
        features: Fact::Known(isa_features()),
        cache_l2_kib: l2,
        cache_l3_kib: l3,
    }
}

fn probe_memory() -> MemoryInfo {
    let mut sys = System::new();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    let (mut total, mut avail) = (sys.total_memory(), sys.available_memory());
    if let Some(cg) = sys.cgroup_limits() {
        // Inside a container the cgroup limit is the real ceiling.
        if cg.total_memory > 0 && cg.total_memory < total {
            total = cg.total_memory;
            avail = cg.free_memory.min(total);
        }
    }
    let known = |b: u64| {
        if b == 0 {
            Fact::Unknown(Reason::Unavailable)
        } else {
            Fact::Known(parse::mib(b))
        }
    };
    MemoryInfo {
        total_mib: known(total),
        available_mib: known(avail),
    }
}

fn probe_storage(dir: Option<&Path>) -> StorageInfo {
    let dir = dir
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .and_then(|d| std::fs::canonicalize(d).ok());
    let Some(dir) = dir else {
        return StorageInfo::unknown(Reason::Unavailable);
    };
    let disks = Disks::new_with_refreshed_list();
    let best = disks
        .list()
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len());
    match best {
        None => StorageInfo::unknown(Reason::Unavailable),
        Some(d) => StorageInfo {
            class: match d.kind() {
                DiskKind::SSD => Fact::Known(StorageClass::Ssd),
                DiskKind::HDD => Fact::Known(StorageClass::Hdd),
                DiskKind::Unknown(_) => Fact::Unknown(Reason::Unavailable),
            },
            free_gib: Fact::Known(d.available_space() / (1024 * 1024 * 1024)),
        },
    }
}

fn gpu_from(i: &GpuInput) -> GpuInfo {
    let keep: Vec<String> = GPU_FEATURES
        .iter()
        .filter(|f| i.features.iter().any(|x| x.eq_ignore_ascii_case(f)))
        .map(|f| (*f).to_owned())
        .collect();
    fn opt<T>(o: Option<T>) -> Fact<T> {
        Fact::from_option(o, Reason::NeedsRenderer)
    }
    GpuInfo {
        vendor: Fact::Known(parse::gpu_vendor_from_id(i.vendor_id)),
        device_class: Fact::Known(parse::gpu_class(&i.device_type)),
        vram_mib: opt(i.vram_mib),
        backend: Fact::Known(parse::gpu_backend(&i.backend)),
        max_texture_2d: opt(i.max_texture_2d),
        max_buffer_mib: opt(i.max_buffer_bytes.map(parse::mib)),
        features: Fact::Known(keep),
        ray_tracing: opt(i.ray_tracing),
        dlss_runtime: opt(i.dlss_runtime),
    }
}

fn display_from(i: &DisplayInput) -> DisplayInfo {
    let nz = |v: u32| Fact::from_option(Some(v).filter(|v| *v > 0), Reason::NeedsRenderer);
    DisplayInfo {
        count: Fact::Known(i.count),
        width: nz(i.width),
        height: nz(i.height),
        refresh_hz: Fact::from_option(
            i.refresh_millihertz
                .map(|m| (m + 500) / 1000)
                .filter(|v| *v > 0),
            Reason::NeedsRenderer,
        ),
        scale_percent: Fact::from_option(
            i.scale_factor
                .filter(|s| s.is_finite() && *s > 0.0 && *s < 16.0)
                .map(|s| (s * 100.0).round() as u32),
            Reason::NeedsRenderer,
        ),
        hdr: Fact::from_option(i.hdr, Reason::NeedsRenderer),
    }
}

fn audio_from(i: AudioInput) -> AudioInfo {
    AudioInfo {
        device_present: Fact::Known(i.device_present),
        channels: Fact::from_option(
            i.channels.filter(|_| i.device_present).map(|c| match c {
                1 => ChannelLayout::Mono,
                2 => ChannelLayout::Stereo,
                6 => ChannelLayout::Surround51,
                8 => ChannelLayout::Surround71,
                _ => ChannelLayout::Other,
            }),
            Reason::NeedsRenderer,
        ),
    }
}

fn input_from(i: InputDevicesInput) -> InputInfo {
    InputInfo {
        gamepad_present: Fact::Known(i.gamepads > 0),
        wheel_present: Fact::from_option(i.wheels.map(|w| w > 0), Reason::NeedsRenderer),
    }
}

/// Probes this machine with the default budget and no renderer data.
#[must_use]
pub fn probe() -> HardwareProfile {
    probe_with(&ProbeInputs::default())
}

/// Probes this machine; `inputs` adds what the renderer, audio and input layers know.
#[must_use]
pub fn probe_with(inputs: &ProbeInputs) -> HardwareProfile {
    probe_on(&Native, inputs)
}

/// The probe over any platform implementation (tests use fakes). A stage that would start after
/// the deadline is skipped and its facts are `Unknown(timeout)`.
#[must_use]
pub fn probe_on(p: &dyn PlatformProbe, inputs: &ProbeInputs) -> HardwareProfile {
    let clock = Clock {
        start: Instant::now(),
        budget: inputs.budget,
    };
    let t = Reason::Timeout;
    macro_rules! stage {
        ($ty:ident, $e:expr) => {
            if clock.left() {
                $e
            } else {
                $ty::unknown(t)
            }
        };
    }
    let mut out = HardwareProfile {
        schema: SCHEMA_VERSION,
        os: stage!(OsInfo, probe_os(p)),
        cpu: stage!(CpuInfo, probe_cpu(p)),
        memory: stage!(MemoryInfo, probe_memory()),
        gpu: stage!(
            GpuInfo,
            inputs
                .gpu
                .as_ref()
                .map_or_else(|| p.gpu_fallback(), gpu_from)
        ),
        display: stage!(
            DisplayInfo,
            inputs
                .display
                .as_ref()
                .map_or_else(|| p.display_fallback(), display_from)
        ),
        audio: stage!(
            AudioInfo,
            inputs.audio.map_or_else(|| p.audio_fallback(), audio_from)
        ),
        input: stage!(
            InputInfo,
            inputs.input.map_or_else(|| p.input_fallback(), input_from)
        ),
        storage: stage!(StorageInfo, probe_storage(inputs.content_dir.as_deref())),
        bench: Benchmarks::unknown(Reason::NotRequested),
        ..HardwareProfile::default()
    };
    out.probe_ms = u32::try_from(clock.start.elapsed().as_millis()).unwrap_or(u32::MAX);
    out
}
