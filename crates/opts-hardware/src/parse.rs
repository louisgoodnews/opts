//! Pure parsers over text. They take strings (never touch the file system) so the Linux,
//! macOS and Windows formats can be unit-tested on any host. Each returns only a *class*:
//! the input text (which may contain model names or identifiers) is dropped by the caller.
#![cfg_attr(
    not(any(target_os = "linux", test)),
    allow(
        dead_code,
        reason = "the Linux parsers are compiled everywhere to be tested everywhere"
    )
)]

use crate::profile::{Arch, CpuVendor, Environment, GpuBackend, GpuClass, GpuVendor, VersionClass};

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

/// Class of a device type name as the renderer reports it (`DiscreteGpu`, `integrated`, ...).
#[must_use]
pub fn gpu_class(s: &str) -> GpuClass {
    let l = s.to_ascii_lowercase();
    if l.contains("discrete") {
        GpuClass::Discrete
    } else if l.contains("integrated") {
        GpuClass::Integrated
    } else if l.contains("virtual") {
        GpuClass::Virtual
    } else if l.contains("cpu") || l.contains("software") {
        GpuClass::Software
    } else {
        GpuClass::Other
    }
}

/// Class of a backend name (`Vulkan`, `Dx12`, `Metal`, `Gl`).
#[must_use]
pub fn gpu_backend(s: &str) -> GpuBackend {
    let l = s.to_ascii_lowercase();
    if l.contains("vulkan") {
        GpuBackend::Vulkan
    } else if l.contains("dx12") || l.contains("d3d12") {
        GpuBackend::Dx12
    } else if l.contains("metal") {
        GpuBackend::Metal
    } else if l.contains("gl") {
        GpuBackend::Gl
    } else {
        GpuBackend::Other
    }
}

/// Does `/proc/cpuinfo` carry the `hypervisor` flag? Only the flag lines are inspected.
#[must_use]
pub fn cpuinfo_hypervisor(text: &str) -> bool {
    text.lines()
        .filter(|l| l.starts_with("flags"))
        .any(|l| l.split_whitespace().any(|w| w == "hypervisor"))
}

/// Container runtime keywords in `/proc/self/cgroup`.
#[must_use]
pub fn cgroup_container(text: &str) -> bool {
    const WORDS: [&str; 6] = ["docker", "kubepods", "lxc", "containerd", "libpod", "crio"];
    text.lines().any(|l| {
        let l = l.to_ascii_lowercase();
        WORDS.iter().any(|w| l.contains(w))
    })
}

/// DMI vendor or product text names a hypervisor (the text itself is dropped).
#[must_use]
pub fn dmi_virtual(text: &str) -> bool {
    const WORDS: [&str; 9] = [
        "qemu",
        "kvm",
        "vmware",
        "virtualbox",
        "xen",
        "bochs",
        "parallels",
        "hyper-v",
        "virtual machine",
    ];
    let l = text.to_ascii_lowercase();
    WORDS.iter().any(|w| l.contains(w))
}

/// Combines the evidence into the environment class (container wins over VM).
#[must_use]
pub fn environment(container: bool, vm: bool) -> Environment {
    if container {
        Environment::Container
    } else if vm {
        Environment::Vm
    } else {
        Environment::Bare
    }
}

/// `32K`, `1024K`, `32M`, `32768K\n` to KiB.
#[must_use]
pub fn cache_kib(s: &str) -> Option<u32> {
    let t = s.trim();
    let (num, mult) = if let Some(n) = t.strip_suffix(['K', 'k']) {
        (n, 1)
    } else if let Some(n) = t.strip_suffix(['M', 'm']) {
        (n, 1024)
    } else {
        (t, 1)
    };
    num.trim().parse::<u32>().ok()?.checked_mul(mult)
}

/// First `WIDTHxHEIGHT` line of a DRM `modes` file (the preferred mode).
#[must_use]
pub fn drm_first_mode(text: &str) -> Option<(u32, u32)> {
    let line = text.lines().next()?;
    let line = line.split(['i', 'p']).next().unwrap_or(line);
    let (w, h) = line.trim().split_once('x')?;
    let (w, h) = (w.parse::<u32>().ok()?, h.parse::<u32>().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

/// Number of gamepad or joystick handlers (`js0`, `js1`) in `/proc/bus/input/devices`; only the
/// `Handlers=` lines are inspected.
#[must_use]
pub fn count_joysticks(text: &str) -> u32 {
    let n = text
        .lines()
        .filter(|l| l.starts_with("H: Handlers="))
        .filter(|l| l.split_whitespace().any(is_js_handler))
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn is_js_handler(w: &str) -> bool {
    w.strip_prefix("js")
        .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Number of sound cards in `/proc/asound/cards` (`--- no soundcards ---` gives 0).
#[must_use]
pub fn count_sound_cards(text: &str) -> u32 {
    let n = text
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            t.split_whitespace()
                .next()
                .is_some_and(|w| w.bytes().all(|b| b.is_ascii_digit()))
                && t.contains('[')
        })
        .count();
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Bytes to whole MiB (rounded down).
#[must_use]
pub const fn mib(bytes: u64) -> u64 {
    bytes / (1024 * 1024)
}

#[cfg(test)]
mod tests;
