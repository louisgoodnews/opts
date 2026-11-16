//! The field table of the hardware profile.


/// One field: JSON path, value type, what decision uses it, where it comes from.
pub struct Field {
    /// Path in the JSON document.
    pub path: &'static str,
    /// Value type or the closed set of values.
    pub value: &'static str,
    /// The decision that needs it.
    pub used_for: &'static str,
    /// Source per platform.
    pub source: &'static str,
}

pub(super) const fn f(
    path: &'static str,
    value: &'static str,
    used_for: &'static str,
    source: &'static str,
) -> Field {
    Field {
        path,
        value,
        used_for,
        source,
    }
}

/// Every field of [`crate::HardwareProfile`] (except `schema` and `probe_ms`, listed separately).
pub const FIELDS: &[Field] = &[
    f("os.family", "enum", "feature gates, DLSS", "compile-time OS constant"),
    f("os.version", "major.minor numbers", "bug reports, workarounds", "`sysinfo` (kernel on Linux), digits only"),
    f("os.display_server", "enum", "window and input behaviour", "Linux: session variables (presence); macOS/Windows: fixed"),
    f("os.environment", "enum", "conservative defaults in VMs and containers", "Linux: cgroup keywords, marker files, hypervisor flag, DMI keywords"),
    f("cpu.arch", "enum", "feature gates", "compile-time constant"),
    f("cpu.vendor", "enum", "report only", "`sysinfo` brand classified, string dropped"),
    f("cpu.physical_cores", "count", "tile radius, worker threads", "`sysinfo`"),
    f("cpu.logical_cores", "count", "tile radius, worker threads", "`std::thread::available_parallelism`"),
    f("cpu.features", "allowlisted names", "SIMD paths", "`std` feature detection"),
    f("cpu.cache_l2_kib", "KiB", "report only", "Linux: sysfs cache entries; elsewhere unknown"),
    f("cpu.cache_l3_kib", "KiB", "report only", "Linux: sysfs cache entries; elsewhere unknown"),
    f("memory.total_mib", "MiB", "tile radius, memory guard", "`sysinfo`, container limit applied"),
    f("memory.available_mib", "MiB", "tile radius, memory guard", "`sysinfo`, container limit applied"),
    f("gpu.vendor", "enum", "tier, DLSS", "renderer vendor id classified; Linux fallback: PCI vendor id; Apple Silicon: by architecture"),
    f("gpu.device_class", "enum", "tier", "renderer device type; Apple Silicon: integrated"),
    f("gpu.vram_mib", "MiB", "tier", "renderer; Linux fallback: amdgpu sysfs"),
    f("gpu.backend", "enum", "DLSS", "renderer"),
    f("gpu.max_texture_2d", "pixels", "texture budget", "renderer limits"),
    f("gpu.max_buffer_mib", "MiB", "mesh budget", "renderer limits"),
    f("gpu.features", "allowlisted names", "format choices", "renderer feature set, filtered by allowlist"),
    f("gpu.ray_tracing", "bool", "DLSS", "renderer"),
    f("gpu.dlss_runtime", "bool", "DLSS", "renderer"),
    f("display.count", "count", "window placement", "renderer; Linux fallback: connected DRM connectors"),
    f("display.width", "pixels", "UI scale", "renderer; Linux fallback: preferred mode"),
    f("display.height", "pixels", "UI scale", "renderer; Linux fallback: preferred mode"),
    f("display.refresh_hz", "Hz", "frame pacing hints", "renderer"),
    f("display.scale_percent", "percent", "UI scale", "renderer"),
    f("display.hdr", "bool", "look", "renderer"),
    f("storage.class", "enum", "tile radius", "`sysinfo` disk kind of the content volume"),
    f("storage.free_gib", "GiB", "cache sizes", "`sysinfo` free space of the content volume"),
    f("audio.device_present", "bool", "audio settings availability", "audio layer; Linux fallback: sound card count"),
    f("audio.channels", "enum", "audio mixer layout", "audio layer"),
    f("input.gamepad_present", "bool", "controls page", "input layer; Linux fallback: `js*` handler count"),
    f("input.wheel_present", "bool", "controls page", "input layer"),
    f("bench.cpu_single_mops", "Mops/s", "tile radius refinement", "micro-benchmark (on request)"),
    f("bench.cpu_multi_mops", "Mops/s", "tile radius refinement", "micro-benchmark (on request)"),
    f("bench.memory_mib_s", "MiB/s", "tile radius refinement", "micro-benchmark (on request)"),
    f("bench.storage_read_mib_s", "MiB/s", "streaming expectations", "micro-benchmark (on request, scratch file deleted)"),
    f("bench.gpu_fill", "arbitrary", "reserved", "not implemented (needs a render device)"),
    f("bench.measured_for_class", "hash", "cache validity", "FNV-1a of the bucketed hardware class"),
];
