//! Unknown-value defaults of the info sections.

use super::*;

macro_rules! all_unknown {
    ($ty:ident { $($f:ident),+ }) => {
        impl $ty {
            /// Every fact unknown for one `reason`.
            #[must_use]
            pub const fn unknown(reason: crate::fact::Reason) -> Self {
                Self { $($f: Fact::Unknown(reason)),+ }
            }
        }
    };
}

all_unknown!(OsInfo {
    family,
    version,
    display_server,
    environment
});

all_unknown!(CpuInfo {
    arch,
    vendor,
    physical_cores,
    logical_cores,
    features,
    cache_l2_kib,
    cache_l3_kib
});

all_unknown!(MemoryInfo {
    total_mib,
    available_mib
});

all_unknown!(GpuInfo {
    vendor,
    device_class,
    vram_mib,
    backend,
    max_texture_2d,
    max_buffer_mib,
    features,
    ray_tracing,
    dlss_runtime
});

all_unknown!(DisplayInfo {
    count,
    width,
    height,
    refresh_hz,
    scale_percent,
    hdr
});

all_unknown!(StorageInfo { class, free_gib });

all_unknown!(AudioInfo {
    device_present,
    channels
});

all_unknown!(InputInfo {
    gamepad_present,
    wheel_present
});

all_unknown!(Benchmarks {
    cpu_single_mops,
    cpu_multi_mops,
    memory_mib_s,
    storage_read_mib_s,
    gpu_fill,
    measured_for_class
});
