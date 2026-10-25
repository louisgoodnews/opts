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

const fn f(
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

fn keys<T: Copy>(all: &[T], key: impl Fn(T) -> &'static str) -> String {
    all.iter()
        .map(|v| format!("`{}`", key(*v)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The generated page.
#[must_use]
pub fn render() -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# Hardware profile fields (generated)\n\n\
         Generated from `crates/opts-hardware/src/reference.rs` by the test \
         `reference_page_is_up_to_date` (`UPDATE_HARDWARE_DOC=1 cargo test -p opts-hardware reference`). \
         Do not edit by hand. The rules behind this list are the [privacy charter](../architecture/hardware-probe.md) \
         ([ADR-0010](../adr/0010-hardware-probe-privacy.md)).\n\n\
         Schema version **{SCHEMA_VERSION}**. Every field below is `known` (a value) or `unknown` with one of the \
         reasons at the end. `probe_ms` (how long the probe took) and `schema` complete the document.\n\n\
         ## Fields\n\n| Field | Value | Used for | Source |\n| :--- | :--- | :--- | :--- |"
    );
    for x in FIELDS {
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} |",
            x.path, x.value, x.used_for, x.source
        );
    }
    let _ = writeln!(s, "\n## Closed value sets\n");
    let sets: [(&str, String); 11] = [
        ("os.family", keys(OsFamily::ALL, OsFamily::key)),
        (
            "os.display_server",
            keys(DisplayServer::ALL, DisplayServer::key),
        ),
        ("os.environment", keys(Environment::ALL, Environment::key)),
        ("cpu.arch", keys(Arch::ALL, Arch::key)),
        ("cpu.vendor", keys(CpuVendor::ALL, CpuVendor::key)),
        ("gpu.vendor", keys(GpuVendor::ALL, GpuVendor::key)),
        ("gpu.device_class", keys(GpuClass::ALL, GpuClass::key)),
        ("gpu.backend", keys(GpuBackend::ALL, GpuBackend::key)),
        ("storage.class", keys(StorageClass::ALL, StorageClass::key)),
        (
            "audio.channels",
            keys(ChannelLayout::ALL, ChannelLayout::key),
        ),
        (
            "cpu.features",
            ISA_FEATURES
                .iter()
                .map(|x| format!("`{x}`"))
                .collect::<Vec<_>>()
                .join(", "),
        ),
    ];
    for (name, v) in sets {
        let _ = writeln!(s, "* `{name}`: {v}");
    }
    let _ = writeln!(
        s,
        "* `gpu.features`: {}",
        GPU_FEATURES
            .iter()
            .map(|x| format!("`{x}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        s,
        "\n## Reasons for unknown values\n\n| Reason | Meaning |\n| :--- | :--- |"
    );
    for r in Reason::ALL {
        let _ = writeln!(s, "| `{}` | {} |", r.key(), r.describe());
    }
    let _ = writeln!(
        s,
        "\n## Files the Linux probe may read\n\nNothing else is opened (test `no_allowlisted_path_names_an_identifier`; \
         every read goes through `allowlist::read`). `*` matches one path segment.\n\n| Path | Used for |\n| :--- | :--- |"
    );
    for (p, why) in allowlist::READ {
        let _ = writeln!(s, "| `{p}` | {why} |");
    }
    let _ = writeln!(
        s,
        "\nExistence only: {}. Directory listings (names): {}.\n",
        allowlist::EXISTS
            .iter()
            .map(|x| format!("`{x}`"))
            .collect::<Vec<_>>()
            .join(", "),
        allowlist::LIST
            .iter()
            .map(|x| format!("`{x}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Path of the generated page, relative to this crate.
    const PAGE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/src/reference/hardware-fields.md"
    );
    use serde_json::Value;

    fn leaves(prefix: &str, v: &Value, out: &mut Vec<String>) {
        let Value::Object(m) = v else { return };
        let is_fact = m.len() == 1 && (m.contains_key("known") || m.contains_key("unknown"));
        if is_fact {
            out.push(prefix.to_owned());
            return;
        }
        for (k, v) in m {
            let p = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{prefix}.{k}")
            };
            leaves(&p, v, out);
        }
    }

    #[test]
    fn field_table_matches_the_serialised_profile() {
        let v = serde_json::to_value(crate::HardwareProfile::default()).unwrap();
        let mut real = Vec::new();
        leaves("", &v, &mut real);
        real.sort();
        let mut listed: Vec<String> = FIELDS.iter().map(|x| x.path.to_owned()).collect();
        listed.sort();
        assert_eq!(real, listed, "FIELDS and the profile structs disagree");
    }

    #[test]
    fn reference_page_is_up_to_date() {
        let want = render();
        if std::env::var_os("UPDATE_HARDWARE_DOC").is_some() {
            std::fs::write(PAGE, &want).unwrap();
            return;
        }
        // A Windows checkout (`core.autocrlf`) has CRLF line ends; the page is compared by content.
        let have = std::fs::read_to_string(PAGE)
            .unwrap_or_else(|e| panic!("read the reference page: {e}"))
            .replace("\r\n", "\n");
        assert_eq!(
            have, want,
            "run: UPDATE_HARDWARE_DOC=1 cargo test -p opts-hardware reference"
        );
    }
}
