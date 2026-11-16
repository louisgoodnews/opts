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

mod devices;
mod environment;
mod gpu;
mod identity;

pub use devices::{cache_kib, count_joysticks, count_sound_cards, mib};
pub use environment::{cgroup_container, cpuinfo_hypervisor, dmi_virtual, environment};
pub use gpu::{drm_first_mode, gpu_backend, gpu_class};
pub use identity::{arch, cpu_vendor, gpu_vendor_from_id, pci_vendor, version_class};

#[cfg(test)]
mod tests;
