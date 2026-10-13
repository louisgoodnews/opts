//! Pure parsers over text. They take strings (never touch the file system) so the Linux,
//! macOS and Windows formats can be unit-tested on any host. Each returns only a *class*:
//! the input text (which may contain model names or identifiers) is dropped by the caller.
#![cfg_attr(
    not(any(target_os = "linux", test)),
    allow(dead_code, reason = "the Linux parsers are compiled everywhere to be tested everywhere")
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
        "qemu", "kvm", "vmware", "virtualbox", "xen", "bochs", "parallels", "hyper-v", "virtual machine",
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
mod tests {
    use super::*;

    #[test]
    fn vendors() {
        assert_eq!(cpu_vendor("GenuineIntel", "Intel(R) Core(TM) i7"), CpuVendor::Intel);
        assert_eq!(cpu_vendor("AuthenticAMD", "AMD Ryzen 5"), CpuVendor::Amd);
        assert_eq!(cpu_vendor("", "Apple M2 Pro"), CpuVendor::Apple);
        assert_eq!(cpu_vendor("0x41", "Cortex-A76"), CpuVendor::Arm);
        assert_eq!(cpu_vendor("", "Mystery"), CpuVendor::Other);
        assert_eq!(pci_vendor("0x10de"), GpuVendor::Nvidia);
        assert_eq!(pci_vendor("0x1002\n"), GpuVendor::Amd);
        assert_eq!(gpu_vendor_from_id(0x8086), GpuVendor::Intel);
        assert_eq!(arch("arm64"), Arch::Aarch64);
    }

    #[test]
    fn versions_keep_two_numbers_only() {
        let v = |s| version_class(s).map(|v| (v.major, v.minor));
        assert_eq!(v("6.2.9-1-cachyos"), Some((6, 2)));
        assert_eq!(v("14.5"), Some((14, 5)));
        assert_eq!(v("11 (22631)"), Some((11, 0)));
        assert_eq!(v("10.0.22631"), Some((10, 0)));
        assert_eq!(v("5.15.0-91-generic"), Some((5, 15)));
        assert_eq!(v("unknown"), None);
    }

    #[test]
    fn classes_of_renderer_text() {
        assert_eq!(gpu_class("DiscreteGpu"), GpuClass::Discrete);
        assert_eq!(gpu_class("IntegratedGpu"), GpuClass::Integrated);
        assert_eq!(gpu_class("Cpu"), GpuClass::Software);
        assert_eq!(gpu_backend("Vulkan"), GpuBackend::Vulkan);
        assert_eq!(gpu_backend("Dx12"), GpuBackend::Dx12);
        assert_eq!(gpu_backend("BrowserWebGpu"), GpuBackend::Other);
    }

    #[test]
    fn environment_evidence() {
        assert!(cpuinfo_hypervisor("processor : 0\nflags : fpu vme hypervisor lm\n"));
        assert!(!cpuinfo_hypervisor("flags : fpu vme lm\nmodel name : hypervisor inc\n"));
        assert!(cgroup_container("0::/docker/abc\n"));
        assert!(cgroup_container("12:cpu:/kubepods/burstable\n"));
        assert!(!cgroup_container("0::/user.slice/session-1.scope\n"));
        assert!(dmi_virtual("QEMU Standard PC"));
        assert!(!dmi_virtual("ASUSTeK COMPUTER INC."));
        assert_eq!(environment(true, true), Environment::Container);
        assert_eq!(environment(false, true), Environment::Vm);
        assert_eq!(environment(false, false), Environment::Bare);
    }

    #[test]
    fn sizes_and_modes() {
        assert_eq!(cache_kib("1024K\n"), Some(1024));
        assert_eq!(cache_kib("32M"), Some(32768));
        assert_eq!(cache_kib("junk"), None);
        assert_eq!(drm_first_mode("2560x1440\n1920x1080\n"), Some((2560, 1440)));
        assert_eq!(drm_first_mode("1920x1080i\n"), Some((1920, 1080)));
        assert_eq!(drm_first_mode(""), None);
        assert_eq!(mib(3 * 1024 * 1024 + 5), 3);
    }

    #[test]
    fn devices() {
        let input = "I: Bus=0003 Vendor=054c\nN: Name=\"Pad\"\nH: Handlers=event5 js0\n\nH: Handlers=kbd event3\nH: Handlers=sysrq js12 event1\nH: Handlers=jsx\n";
        assert_eq!(count_joysticks(input), 2);
        assert_eq!(count_sound_cards(" 0 [PCH            ]: HDA-Intel - HDA Intel PCH\n                      HDA Intel PCH at 0x1\n"), 1);
        assert_eq!(count_sound_cards("--- no soundcards ---\n"), 0);
    }
}
