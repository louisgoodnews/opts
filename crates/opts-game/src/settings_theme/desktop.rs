//! The desktop's own answers: the reduced-motion preference and the colour scheme, read with
//! `gsettings` on Linux (best effort).

use opts_gui::theme::ThemeKind;

/// Whether the desktop asks for reduced motion (best effort, Linux/GNOME only; `false` when the
/// answer is not available). Read once at start-up. `OPTS_REDUCED_MOTION=1` forces it (tests,
/// other desktops).
pub fn os_reduced_motion_hint() -> bool {
    if std::env::var("OPTS_REDUCED_MOTION").is_ok_and(|v| v == "1" || v == "true") {
        return true;
    }
    #[cfg(target_os = "linux")]
    {
        // `enable-animations` is false when the user turned animations off.
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .output();
        if let Ok(out) = out {
            return String::from_utf8_lossy(&out.stdout).trim() == "false";
        }
    }
    false
}

/// The colour scheme a `gsettings` answer (`'prefer-dark'`, `'prefer-light'`, `'default'`) means;
/// `'default'` is "no preference", so nothing is reported (the theme then falls back to dark).
pub fn parse_gsettings_scheme(answer: &str) -> Option<ThemeKind> {
    match answer.trim().trim_matches('\'') {
        "prefer-dark" => Some(ThemeKind::Dark),
        "prefer-light" => Some(ThemeKind::Light),
        _ => None,
    }
}

/// Asks the desktop for its colour-scheme preference (best effort, Linux/GNOME `gsettings`).
/// Needed because winit does not report a window theme on every desktop (it reports none on
/// Wayland/GNOME at start-up).
pub fn os_color_scheme_hint() -> Option<ThemeKind> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok()?;
        parse_gsettings_scheme(&String::from_utf8_lossy(&out.stdout))
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
