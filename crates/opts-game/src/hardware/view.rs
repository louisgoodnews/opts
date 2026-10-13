//! The profile as lines of the settings page: message key plus named values. Unknown facts show
//! the localised reason (`hw-reason-<key>`); class names are the technical keys of the crate.

use opts_gui::l10n::Localization;
use opts_hardware::recommend::{
    dlss_available, recommended_look, recommended_texture_filtering, recommended_tile_radius,
    recommended_ui_scale, Availability, MapStats,
};
use opts_hardware::{Fact, HardwareProfile};

/// One page line: message key and its variables.
pub type Line = (&'static str, Vec<(&'static str, String)>);

fn fx<T>(f: &Fact<T>, loc: &Localization, show: impl Fn(&T) -> String) -> String {
    match f {
        Fact::Known(v) => show(v),
        Fact::Unknown(r) => loc.t(&format!("hw-reason-{}", r.key())),
    }
}

fn mib_as_gb(m: &u64) -> String {
    format!("{:.1}", *m as f64 / 1024.0)
}

/// The summary lines of a profile.
#[must_use]
pub fn lines(p: &HardwareProfile, loc: &Localization) -> Vec<Line> {
    let n = |v: &u32| v.to_string();
    let os = [
        fx(&p.os.family, loc, |v| v.key().to_owned()),
        fx(&p.os.version, loc, |v| format!("{}.{}", v.major, v.minor)),
        fx(&p.os.display_server, loc, |v| v.key().to_owned()),
        fx(&p.os.environment, loc, |v| v.key().to_owned()),
    ]
    .join(", ");
    let dlss = match dlss_available(p) {
        Availability::Yes => loc.t("hw-dlss-yes"),
        Availability::No(r) => loc.t(&format!("hw-dlss-no-{}", r.key())),
        Availability::Unknown => loc.t("hw-dlss-unknown"),
    };
    vec![
        ("settings-info-hw-os", vec![("os", os)]),
        (
            "settings-info-hw-cpu",
            vec![
                ("vendor", fx(&p.cpu.vendor, loc, |v| v.key().to_owned())),
                ("physical", fx(&p.cpu.physical_cores, loc, n)),
                ("logical", fx(&p.cpu.logical_cores, loc, n)),
            ],
        ),
        (
            "settings-info-hw-memory",
            vec![
                ("available", fx(&p.memory.available_mib, loc, mib_as_gb)),
                ("total", fx(&p.memory.total_mib, loc, mib_as_gb)),
            ],
        ),
        (
            "settings-info-hw-gpu",
            vec![
                ("vendor", fx(&p.gpu.vendor, loc, |v| v.key().to_owned())),
                ("class", fx(&p.gpu.device_class, loc, |v| v.key().to_owned())),
                ("vram", fx(&p.gpu.vram_mib, loc, mib_as_gb)),
                ("backend", fx(&p.gpu.backend, loc, |v| v.key().to_owned())),
            ],
        ),
        (
            "settings-info-hw-display",
            vec![
                ("width", fx(&p.display.width, loc, n)),
                ("height", fx(&p.display.height, loc, n)),
                ("count", fx(&p.display.count, loc, n)),
            ],
        ),
        (
            "settings-info-hw-storage",
            vec![
                ("class", fx(&p.storage.class, loc, |v| v.key().to_owned())),
                ("free", fx(&p.storage.free_gib, loc, |v| v.to_string())),
            ],
        ),
        (
            "settings-info-hw-recs",
            vec![
                ("radius", recommended_tile_radius(p, &MapStats::default()).to_string()),
                ("filter", recommended_texture_filtering(p).to_owned()),
                ("look", recommended_look(p).to_owned()),
                ("scale", recommended_ui_scale(p).to_string()),
            ],
        ),
        ("settings-info-hw-dlss", vec![("dlss", dlss)]),
    ]
}
