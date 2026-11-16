//! Device probes: memory, storage, GPU, display, audio and input.

use super::*;

pub(super) fn probe_memory() -> MemoryInfo {
    let mut sys = System::new();
    sys.refresh_memory_specifics(MemoryRefreshKind::nothing().with_ram());
    let (mut total, mut avail) = (sys.total_memory(), sys.available_memory());
    if let Some(cg) = sys.cgroup_limits() {
        // Inside a container the cgroup limit is the real ceiling.
        if cg.total_memory > 0 && cg.total_memory < total {
            total = cg.total_memory;
            avail = cg.free_memory.min(total);
        }
    }
    let known = |b: u64| {
        if b == 0 {
            Fact::Unknown(Reason::Unavailable)
        } else {
            Fact::Known(parse::mib(b))
        }
    };
    MemoryInfo {
        total_mib: known(total),
        available_mib: known(avail),
    }
}

pub(super) fn probe_storage(dir: Option<&Path>) -> StorageInfo {
    let dir = dir
        .map(Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok())
        .and_then(|d| std::fs::canonicalize(d).ok());
    let Some(dir) = dir else {
        return StorageInfo::unknown(Reason::Unavailable);
    };
    let disks = Disks::new_with_refreshed_list();
    let best = disks
        .list()
        .iter()
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len());
    match best {
        None => StorageInfo::unknown(Reason::Unavailable),
        Some(d) => StorageInfo {
            class: match d.kind() {
                DiskKind::SSD => Fact::Known(StorageClass::Ssd),
                DiskKind::HDD => Fact::Known(StorageClass::Hdd),
                DiskKind::Unknown(_) => Fact::Unknown(Reason::Unavailable),
            },
            free_gib: Fact::Known(d.available_space() / (1024 * 1024 * 1024)),
        },
    }
}

pub(super) fn gpu_from(i: &GpuInput) -> GpuInfo {
    let keep: Vec<String> = GPU_FEATURES
        .iter()
        .filter(|f| i.features.iter().any(|x| x.eq_ignore_ascii_case(f)))
        .map(|f| (*f).to_owned())
        .collect();
    fn opt<T>(o: Option<T>) -> Fact<T> {
        Fact::from_option(o, Reason::NeedsRenderer)
    }
    GpuInfo {
        vendor: Fact::Known(parse::gpu_vendor_from_id(i.vendor_id)),
        device_class: Fact::Known(parse::gpu_class(&i.device_type)),
        vram_mib: opt(i.vram_mib),
        backend: Fact::Known(parse::gpu_backend(&i.backend)),
        max_texture_2d: opt(i.max_texture_2d),
        max_buffer_mib: opt(i.max_buffer_bytes.map(parse::mib)),
        features: Fact::Known(keep),
        ray_tracing: opt(i.ray_tracing),
        dlss_runtime: opt(i.dlss_runtime),
    }
}

pub(super) fn display_from(i: &DisplayInput) -> DisplayInfo {
    let nz = |v: u32| Fact::from_option(Some(v).filter(|v| *v > 0), Reason::NeedsRenderer);
    DisplayInfo {
        count: Fact::Known(i.count),
        width: nz(i.width),
        height: nz(i.height),
        refresh_hz: Fact::from_option(
            i.refresh_millihertz
                .map(|m| (m + 500) / 1000)
                .filter(|v| *v > 0),
            Reason::NeedsRenderer,
        ),
        scale_percent: Fact::from_option(
            i.scale_factor
                .filter(|s| s.is_finite() && *s > 0.0 && *s < 16.0)
                .map(|s| (s * 100.0).round() as u32),
            Reason::NeedsRenderer,
        ),
        hdr: Fact::from_option(i.hdr, Reason::NeedsRenderer),
    }
}

pub(super) fn audio_from(i: AudioInput) -> AudioInfo {
    AudioInfo {
        device_present: Fact::Known(i.device_present),
        channels: Fact::from_option(
            i.channels.filter(|_| i.device_present).map(|c| match c {
                1 => ChannelLayout::Mono,
                2 => ChannelLayout::Stereo,
                6 => ChannelLayout::Surround51,
                8 => ChannelLayout::Surround71,
                _ => ChannelLayout::Other,
            }),
            Reason::NeedsRenderer,
        ),
    }
}

pub(super) fn input_from(i: InputDevicesInput) -> InputInfo {
    InputInfo {
        gamepad_present: Fact::Known(i.gamepads > 0),
        wheel_present: Fact::from_option(i.wheels.map(|w| w > 0), Reason::NeedsRenderer),
    }
}
