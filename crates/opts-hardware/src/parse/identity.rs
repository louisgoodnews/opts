//! Classes of CPU vendor, architecture, OS version and PCI vendor identifiers.

use crate::profile::{Arch, CpuVendor, GpuVendor, VersionClass};

/// Classifies a CPU vendor or brand string.
#[must_use]
pub fn cpu_vendor(vendor_id: &str, brand: &str) -> CpuVendor {
    let both = format!("{vendor_id} {brand}").to_ascii_lowercase();
    if both.contains("genuineintel") || both.contains("intel") {
        CpuVendor::Intel
    } else if both.contains("authenticamd") || both.contains("amd") {
        CpuVendor::Amd
    } else if both.contains("apple") {
        CpuVendor::Apple
    } else if both.contains("arm") || both.contains("qualcomm") || both.contains("cortex") {
        CpuVendor::Arm
    } else {
        CpuVendor::Other
    }
}

/// Architecture class of `std::env::consts::ARCH`-style text.
#[must_use]
pub fn arch(s: &str) -> Arch {
    match s {
        "x86_64" | "amd64" | "x64" => Arch::X86_64,
        "aarch64" | "arm64" => Arch::Aarch64,
        _ => Arch::Other,
    }
}

/// `major.minor` from text such as `6.2.9-1-cachyos`, `14.5`, `11 (22631)` or `10.0.22631`.
/// Everything after the first two numbers (build, distribution, name) is dropped.
#[must_use]
pub fn version_class(s: &str) -> Option<VersionClass> {
    let lead: String = s
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = lead.split('.').filter(|p| !p.is_empty());
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    Some(VersionClass { major, minor })
}

/// PCI vendor id (`0x10de`) to a GPU vendor class.
#[must_use]
pub fn pci_vendor(id: &str) -> GpuVendor {
    let t = id.trim().trim_start_matches("0x").to_ascii_lowercase();
    match t.as_str() {
        "10de" => GpuVendor::Nvidia,
        "1002" | "1022" => GpuVendor::Amd,
        "8086" => GpuVendor::Intel,
        "106b" => GpuVendor::Apple,
        "5143" => GpuVendor::Qualcomm,
        _ => GpuVendor::Other,
    }
}

/// wgpu-style vendor id (the same PCI numbers) or name to a class.
#[must_use]
pub fn gpu_vendor_from_id(id: u32) -> GpuVendor {
    pci_vendor(&format!("{id:x}"))
}
