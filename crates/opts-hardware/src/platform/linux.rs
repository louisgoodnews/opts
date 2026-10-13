//! Linux, any distribution (glibc or musl), containers and VMs. Reads only the allowlisted
//! generic kernel interfaces of [`crate::allowlist`]; every missing or unreadable file gives an
//! `Unknown` with a reason. No root, no systemd/udev/dbus, no external program.

use super::PlatformProbe;
use crate::allowlist;
use crate::fact::{Fact, Reason};
use crate::parse;
use crate::profile::{
    AudioInfo, DisplayInfo, DisplayServer, Environment, GpuInfo, InputInfo,
};

/// The probe of this platform.
pub struct Native;

fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

/// Session type from the standard variables (presence and the value `wayland`/`x11` only).
fn display_server_from(session: Option<&str>, wayland: bool, x11: bool) -> DisplayServer {
    match session {
        Some("wayland") => DisplayServer::Wayland,
        Some("x11") => DisplayServer::X11,
        _ if wayland => DisplayServer::Wayland,
        _ if x11 => DisplayServer::X11,
        _ => DisplayServer::None,
    }
}

fn cache_sizes(entries: &[String]) -> (Fact<u32>, Fact<u32>) {
    let base = "/sys/devices/system/cpu/cpu0/cache";
    let (mut l2, mut l3) = (None, None);
    for e in entries.iter().filter(|e| e.starts_with("index")) {
        let get = |f: &str| allowlist::read(&format!("{base}/{e}/{f}")).ok();
        let (Some(level), Some(kind), Some(size)) = (get("level"), get("type"), get("size")) else {
            continue;
        };
        if kind.trim() == "Instruction" {
            continue;
        }
        match level.trim() {
            "2" => l2 = parse::cache_kib(&size),
            "3" => l3 = parse::cache_kib(&size),
            _ => {}
        }
    }
    (
        Fact::from_option(l2, Reason::Unavailable),
        Fact::from_option(l3, Reason::Unavailable),
    )
}

impl PlatformProbe for Native {
    fn display_server(&self) -> Fact<DisplayServer> {
        let session = std::env::var("XDG_SESSION_TYPE").ok();
        Fact::Known(display_server_from(
            session.as_deref(),
            env_set("WAYLAND_DISPLAY"),
            env_set("DISPLAY"),
        ))
    }

    fn environment(&self) -> Fact<Environment> {
        let cgroup = allowlist::read("/proc/self/cgroup").ok();
        let container = allowlist::exists("/.dockerenv")
            || allowlist::exists("/run/.containerenv")
            || cgroup.as_deref().is_some_and(parse::cgroup_container);
        let vm = allowlist::read("/proc/cpuinfo")
            .ok()
            .is_some_and(|t| parse::cpuinfo_hypervisor(&t))
            || ["sys_vendor", "product_name"].iter().any(|f| {
                allowlist::read(&format!("/sys/class/dmi/id/{f}"))
                    .ok()
                    .is_some_and(|t| parse::dmi_virtual(&t))
            });
        Fact::Known(parse::environment(container, vm))
    }

    fn cpu_caches_kib(&self) -> (Fact<u32>, Fact<u32>) {
        match allowlist::list("/sys/devices/system/cpu/cpu0/cache") {
            Ok(entries) => cache_sizes(&entries),
            Err(r) => (Fact::Unknown(r), Fact::Unknown(r)),
        }
    }

    fn gpu_fallback(&self) -> GpuInfo {
        let mut g = super::gpu_needs_renderer();
        let Ok(entries) = allowlist::list("/sys/class/drm") else {
            return g;
        };
        // The first card (`cardN`, no connector suffix) with a readable vendor id wins.
        for card in entries.iter().filter(|e| e.starts_with("card") && !e.contains('-')) {
            let Ok(id) = allowlist::read(&format!("/sys/class/drm/{card}/device/vendor")) else {
                continue;
            };
            g.vendor = Fact::Known(parse::pci_vendor(&id));
            if let Some(b) = allowlist::read(&format!(
                "/sys/class/drm/{card}/device/mem_info_vram_total"
            ))
            .ok()
            .and_then(|t| t.trim().parse::<u64>().ok())
            {
                g.vram_mib = Fact::Known(parse::mib(b));
            }
            break;
        }
        g
    }

    fn display_fallback(&self) -> DisplayInfo {
        let mut d = super::display_needs_renderer();
        let Ok(entries) = allowlist::list("/sys/class/drm") else {
            return d;
        };
        let mut count = 0u32;
        let mut first = None;
        for c in entries.iter().filter(|e| e.starts_with("card") && e.contains('-')) {
            let connected = allowlist::read(&format!("/sys/class/drm/{c}/status"))
                .is_ok_and(|s| s.trim() == "connected");
            if !connected {
                continue;
            }
            count += 1;
            if first.is_none() {
                first = allowlist::read(&format!("/sys/class/drm/{c}/modes"))
                    .ok()
                    .and_then(|t| parse::drm_first_mode(&t));
            }
        }
        d.count = Fact::Known(count);
        if let Some((w, h)) = first {
            d.width = Fact::Known(w);
            d.height = Fact::Known(h);
        }
        d
    }

    fn audio_fallback(&self) -> AudioInfo {
        let mut a = super::audio_needs_renderer();
        match allowlist::read("/proc/asound/cards") {
            Ok(t) => a.device_present = Fact::Known(parse::count_sound_cards(&t) > 0),
            Err(r) => a.device_present = Fact::Unknown(r),
        }
        a
    }

    fn input_fallback(&self) -> InputInfo {
        let mut i = super::input_needs_renderer();
        match allowlist::read("/proc/bus/input/devices") {
            Ok(t) => i.gamepad_present = Fact::Known(parse::count_joysticks(&t) > 0),
            Err(r) => i.gamepad_present = Fact::Unknown(r),
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_type_wins_then_sockets() {
        assert_eq!(display_server_from(Some("wayland"), false, true), DisplayServer::Wayland);
        assert_eq!(display_server_from(Some("tty"), false, true), DisplayServer::X11);
        assert_eq!(display_server_from(None, true, true), DisplayServer::Wayland);
        assert_eq!(display_server_from(None, false, false), DisplayServer::None);
    }

    #[test]
    fn probes_never_panic_here() {
        let n = Native;
        let _ = (n.display_server(), n.environment(), n.cpu_caches_kib());
        let _ = (n.gpu_fallback(), n.display_fallback(), n.audio_fallback(), n.input_fallback());
    }
}
