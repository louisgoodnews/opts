//! Tier-dependent recommendation tests.

use super::*;

#[test]
fn filtering_and_look_follow_the_tier() {
    let high = machine(
        (Nvidia, Discrete, Some(12 * 1024)),
        8192,
        8,
        Ssd,
        (1920, 1080),
    );
    let mid = machine(
        (Nvidia, Discrete, Some(4 * 1024)),
        8192,
        8,
        Ssd,
        (1920, 1080),
    );
    let low = machine((Intel, Integrated, None), 8192, 8, Ssd, (1920, 1080));
    let mut soft = machine(
        (GpuVendor::Software, GpuClass::Software, None),
        8192,
        8,
        Ssd,
        (1920, 1080),
    );
    assert_eq!(gpu_tier(&high), GpuTier::High);
    assert_eq!(gpu_tier(&mid), GpuTier::Mid);
    assert_eq!(gpu_tier(&low), GpuTier::Low);
    assert_eq!(gpu_tier(&HardwareProfile::default()), GpuTier::Unknown);
    assert_eq!(recommended_texture_filtering(&high), "16");
    assert_eq!(recommended_texture_filtering(&mid), "8");
    assert_eq!(recommended_texture_filtering(&low), "4");
    assert_eq!(recommended_texture_filtering(&soft), "off");
    assert_eq!(
        recommended_texture_filtering(&HardwareProfile::default()),
        "8"
    );
    assert_eq!(recommended_look(&high), "vivid");
    assert_eq!(recommended_look(&low), "bevy");
    soft.gpu.device_class = Fact::Known(Discrete);
    soft.gpu.vendor = Fact::Known(Nvidia);
    soft.gpu.vram_mib = Fact::Known(16 * 1024);
    soft.display.hdr = Fact::Known(true);
    assert_eq!(recommended_look(&soft), "natural");
}

#[test]
fn apple_silicon_tier_follows_unified_memory() {
    let mut p = machine(
        (GpuVendor::Apple, Integrated, None),
        8192,
        8,
        Ssd,
        (3024, 1964),
    );
    p.memory.total_mib = Fact::Known(16 * 1024);
    assert_eq!(gpu_tier(&p), GpuTier::High);
    p.memory.total_mib = Fact::Known(8 * 1024);
    assert_eq!(gpu_tier(&p), GpuTier::Mid);
    p.memory.total_mib = Fact::Known(4 * 1024);
    assert_eq!(gpu_tier(&p), GpuTier::Low);
}

#[test]
fn ui_scale_table() {
    let at = |w, h, scale: Option<u32>| {
        let mut p = HardwareProfile::default();
        p.display.width = Fact::Known(w);
        p.display.height = Fact::Known(h);
        if let Some(s) = scale {
            p.display.scale_percent = Fact::Known(s);
        }
        recommended_ui_scale(&p)
    };
    assert_eq!(at(1920, 1080, None), 100);
    assert_eq!(at(2560, 1440, Some(100)), 125);
    assert_eq!(at(3840, 2160, Some(100)), 150);
    assert_eq!(at(3840, 2160, Some(200)), 100, "the OS already scaled it");
    assert_eq!(at(2880, 1800, Some(200)), 100);
    assert_eq!(recommended_ui_scale(&HardwareProfile::default()), 100);
}

#[test]
fn dlss_table() {
    let mut p = machine(
        (Nvidia, Discrete, Some(8 * 1024)),
        8192,
        8,
        Ssd,
        (1920, 1080),
    );
    assert_eq!(
        dlss_available(&p),
        Availability::Unknown,
        "ray tracing unknown"
    );
    p.gpu.ray_tracing = Fact::Known(true);
    p.gpu.backend = Fact::Known(GpuBackend::Vulkan);
    assert_eq!(dlss_available(&p), Availability::Yes);
    p.gpu.dlss_runtime = Fact::Known(false);
    assert_eq!(dlss_available(&p), Availability::No(DlssNo::NoRuntime));
    p.gpu.dlss_runtime = Fact::Known(true);
    p.gpu.ray_tracing = Fact::Known(false);
    assert_eq!(dlss_available(&p), Availability::No(DlssNo::NoRayTracing));
    p.gpu.ray_tracing = Fact::Known(true);
    p.gpu.backend = Fact::Known(GpuBackend::Metal);
    assert_eq!(
        dlss_available(&p),
        Availability::No(DlssNo::BackendUnsupported)
    );
    p.gpu.backend = Fact::Known(GpuBackend::Vulkan);
    p.os.family = Fact::Known(OsFamily::Macos);
    assert_eq!(dlss_available(&p), Availability::No(DlssNo::OsUnsupported));
    p.os.family = Fact::Known(OsFamily::Windows);
    p.gpu.vendor = Fact::Known(Amd);
    assert_eq!(dlss_available(&p), Availability::No(DlssNo::NotNvidia));
    assert_eq!(
        dlss_available(&HardwareProfile::default()),
        Availability::Unknown
    );
}
