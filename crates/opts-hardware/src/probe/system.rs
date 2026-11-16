//! System probes: the OS, the CPU and its features.

use super::*;

pub(super) struct Clock {
    pub(super) start: Instant,
    pub(super) budget: std::time::Duration,
}

impl Clock {
    pub(super) fn left(&self) -> bool {
        self.start.elapsed() < self.budget
    }
}

pub(super) fn os_family() -> OsFamily {
    match std::env::consts::OS {
        "linux" => OsFamily::Linux,
        "macos" => OsFamily::Macos,
        "windows" => OsFamily::Windows,
        _ => OsFamily::Other,
    }
}

pub(super) fn probe_os(p: &dyn PlatformProbe) -> OsInfo {
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

pub(super) fn isa_features() -> Vec<String> {
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

pub(super) fn probe_cpu(p: &dyn PlatformProbe) -> CpuInfo {
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
