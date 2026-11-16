//! Rendering of the hardware reference page.

use super::*;

pub(super) fn keys<T: Copy>(all: &[T], key: impl Fn(T) -> &'static str) -> String {
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
    render_sets(&mut s);
    render_reasons_and_allowlists(&mut s);
    s
}

pub(super) fn render_sets(s: &mut String) {
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
}

pub(super) fn render_reasons_and_allowlists(s: &mut String) {
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
}
