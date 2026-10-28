//! Unit tests for `super` (`parse.rs`).

use super::*;

#[test]
fn vendors() {
    assert_eq!(
        cpu_vendor("GenuineIntel", "Intel(R) Core(TM) i7"),
        CpuVendor::Intel
    );
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
    assert!(cpuinfo_hypervisor(
        "processor : 0\nflags : fpu vme hypervisor lm\n"
    ));
    assert!(!cpuinfo_hypervisor(
        "flags : fpu vme lm\nmodel name : hypervisor inc\n"
    ));
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
