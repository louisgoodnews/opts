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

mod devices;
mod system;
use devices::*;
use system::*;

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
