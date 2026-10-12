//! Which palette is active and how it is chosen: [`ThemeKind`] (the palettes), [`ThemeMode`]
//! (the user's choice: Dark, Light, System, Auto by clock) and the pure [`theme_for`] clock rule.
//!
//! See `docs/src/roadmap/phase-9/theme-auto-switch.md` for the investigation behind the modes.

use chrono::NaiveTime;

/// A concrete palette family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ThemeKind {
    /// The default: dark surfaces, light text.
    #[default]
    Dark,
    /// Light surfaces, dark text.
    Light,
    /// The accessibility theme: text >= 7:1, controls >= 4.5:1 (dark base). Never chosen
    /// automatically.
    HighContrast,
}

impl ThemeKind {
    /// `true` when the surfaces are dark (dark and high-contrast).
    pub const fn is_dark(self) -> bool {
        !matches!(self, Self::Light)
    }

    /// Stable key used in settings and theme files.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",   // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Light => "light", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::HighContrast => "high_contrast", // l10n-ok: stored identifier or diagnostic, not UI copy
        }
    }

    /// Parses [`ThemeKind::key`] (case-insensitive).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark), // l10n-ok: stored identifier or diagnostic, not UI copy
            "light" => Some(Self::Light), // l10n-ok: stored identifier or diagnostic, not UI copy
            "high_contrast" | "high-contrast" | "highcontrast" => Some(Self::HighContrast), // l10n-ok: stored identifier or diagnostic, not UI copy
            _ => None,
        }
    }
}

/// How the application picks between dark and light (the user's choice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ThemeMode {
    /// Always dark (the default, decision D-UI-02).
    #[default]
    Dark,
    /// Always light.
    Light,
    /// Follow the operating system; dark when it reports nothing.
    System,
    /// Light by day, dark by night using the device's local time ([`theme_for`]).
    #[cfg_attr(feature = "serde", serde(rename = "auto"))]
    AutoClock,
}

impl ThemeMode {
    /// All modes in display order.
    pub const ALL: [ThemeMode; 4] = [Self::Dark, Self::Light, Self::System, Self::AutoClock];

    /// Stable key used in the `theme_mode` setting.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Dark => "dark",   // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::Light => "light", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::System => "system", // l10n-ok: stored identifier or diagnostic, not UI copy
            Self::AutoClock => "auto", // l10n-ok: stored identifier or diagnostic, not UI copy
        }
    }

    /// Parses [`ThemeMode::key`] (case-insensitive); `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark), // l10n-ok: stored identifier or diagnostic, not UI copy
            "light" => Some(Self::Light), // l10n-ok: stored identifier or diagnostic, not UI copy
            "system" => Some(Self::System), // l10n-ok: stored identifier or diagnostic, not UI copy
            "auto" | "auto_clock" | "autoclock" => Some(Self::AutoClock), // l10n-ok: stored identifier or diagnostic, not UI copy
            _ => None,
        }
    }

    /// The message key of the display name (`crate::l10n::keys::theme`).
    pub const fn label_key(self) -> &'static str {
        use crate::l10n::keys::theme as k;
        match self {
            Self::Dark => k::MODE_DARK,
            Self::Light => k::MODE_LIGHT,
            Self::System => k::MODE_SYSTEM,
            Self::AutoClock => k::MODE_AUTO,
        }
    }

    /// Maps the legacy boolean `is_dark_theme` setting (used until M-0061) to a mode.
    pub const fn from_legacy_is_dark(is_dark: bool) -> Self {
        if is_dark {
            Self::Dark
        } else {
            Self::Light
        }
    }

    /// `true` when the mode can change the theme without the user doing anything.
    pub const fn is_automatic(self) -> bool {
        matches!(self, Self::System | Self::AutoClock)
    }
}

/// Default time the light theme starts in *Auto (clock)*: 07:00.
pub const DEFAULT_LIGHT_FROM: NaiveTime = match NaiveTime::from_hms_opt(7, 0, 0) {
    Some(t) => t,
    None => NaiveTime::MIN,
};
/// Default time the dark theme starts in *Auto (clock)*: 19:00.
pub const DEFAULT_DARK_FROM: NaiveTime = match NaiveTime::from_hms_opt(19, 0, 0) {
    Some(t) => t,
    None => NaiveTime::MIN,
};

/// The theme *Auto (clock)* shows at local time `time`: light from `light_from` (inclusive) until
/// `dark_from` (exclusive), dark otherwise.
///
/// The rule is interval-based, never "did we cross the minute X", so it is correct when a daylight
/// saving jump skips or repeats the switch time. The interval may wrap midnight (`light_from` after
/// `dark_from` means light overnight). With `light_from == dark_from` there is no light interval
/// and the answer is always [`ThemeKind::Dark`] (the default theme).
pub fn theme_for(time: NaiveTime, light_from: NaiveTime, dark_from: NaiveTime) -> ThemeKind {
    let light = if light_from == dark_from {
        false
    } else if light_from < dark_from {
        time >= light_from && time < dark_from
    } else {
        // wraps midnight: light from `light_from` through midnight until `dark_from`
        time >= light_from || time < dark_from
    };
    if light {
        ThemeKind::Light
    } else {
        ThemeKind::Dark
    }
}

/// Parses `HH:MM` (also `H:MM`) into a time of day.
pub fn parse_time(s: &str) -> Option<NaiveTime> {
    let (h, m) = s.trim().split_once(':')?;
    NaiveTime::from_hms_opt(h.parse().ok()?, m.parse().ok()?, 0)
}

/// Formats a time of day as `HH:MM`.
pub fn format_time(t: NaiveTime) -> String {
    use chrono::Timelike;
    format!("{:02}:{:02}", t.hour(), t.minute())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, NaiveDate, NaiveDateTime};

    fn t(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).expect("valid time")
    }

    #[test]
    fn default_hours_split_the_day() {
        let (l, d) = (DEFAULT_LIGHT_FROM, DEFAULT_DARK_FROM);
        assert_eq!(theme_for(t(6, 59), l, d), ThemeKind::Dark);
        assert_eq!(theme_for(t(7, 0), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(12, 0), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(18, 59), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(19, 0), l, d), ThemeKind::Dark);
        assert_eq!(theme_for(t(0, 0), l, d), ThemeKind::Dark);
        assert_eq!(theme_for(t(23, 59), l, d), ThemeKind::Dark);
    }

    #[test]
    fn midnight_wrap_means_light_overnight() {
        // light from 22:00 until 06:00 (a night-shift user)
        let (l, d) = (t(22, 0), t(6, 0));
        assert_eq!(theme_for(t(21, 59), l, d), ThemeKind::Dark);
        assert_eq!(theme_for(t(22, 0), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(0, 0), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(5, 59), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(6, 0), l, d), ThemeKind::Dark);
        assert_eq!(theme_for(t(12, 0), l, d), ThemeKind::Dark);
    }

    #[test]
    fn equal_times_are_always_dark() {
        for h in 0..24 {
            assert_eq!(theme_for(t(h, 0), t(9, 0), t(9, 0)), ThemeKind::Dark);
        }
    }

    /// Local wall-clock minutes of a Europe/Berlin switch day, derived from real UTC instants and
    /// the fixed offset schedule (the clocks change at 01:00 UTC), so no time-zone database is
    /// needed: on the spring day 02:00-03:00 never occurs, on the autumn day 02:00-03:00 occurs
    /// twice.
    fn local_minutes(spring: bool) -> Vec<NaiveTime> {
        let day = if spring {
            NaiveDate::from_ymd_opt(2026, 3, 29)
        } else {
            NaiveDate::from_ymd_opt(2026, 10, 25)
        }
        .expect("date");
        let switch_utc = day.and_hms_opt(1, 0, 0).expect("time");
        let (before, after) = if spring { (1, 2) } else { (2, 1) };
        let mut utc = NaiveDateTime::new(day, t(0, 0)) - Duration::hours(before);
        let mut out = Vec::new();
        for _ in 0..26 * 60 {
            let off = if utc < switch_utc { before } else { after };
            let local = utc + Duration::hours(off);
            if local.date() == day {
                out.push(local.time());
            }
            utc += Duration::minutes(1);
        }
        out
    }

    #[test]
    fn dst_days_neither_skip_nor_flap() {
        for spring in [true, false] {
            let minutes = local_minutes(spring);
            // 23 h on the spring day, 25 h on the autumn day
            let expected = if spring { 23 * 60 } else { 25 * 60 };
            assert_eq!(minutes.len(), expected, "spring={spring}");
            for (l, d) in [
                (t(7, 0), t(19, 0)),
                (t(2, 30), t(19, 0)),
                (t(7, 0), t(2, 30)),
            ] {
                let mut flips = 0;
                let mut prev = theme_for(minutes[0], l, d);
                for &m in &minutes[1..] {
                    let now = theme_for(m, l, d);
                    // the same wall-clock time always gives the same answer (repeated hour)
                    assert_eq!(now, theme_for(m, l, d));
                    if now != prev {
                        flips += 1;
                    }
                    prev = now;
                }
                // a skipped or repeated hour adds no switch: at most the two real ones (plus
                // the repeated hour re-entering the same interval)
                assert!(flips <= 4, "spring={spring} {l}/{d}: {flips} flips");
            }
        }
    }

    #[test]
    fn skipped_switch_time_still_switches() {
        // dark_from 02:30 does not exist on the spring-forward day: 01:59 -> 03:00
        let (l, d) = (t(7, 0), t(2, 30));
        // light 07:00 -> 02:30 wraps midnight: light until 02:30, which does not exist today
        assert_eq!(theme_for(t(1, 59), l, d), ThemeKind::Light);
        assert_eq!(theme_for(t(3, 0), l, d), ThemeKind::Dark);
    }

    #[test]
    fn keys_roundtrip() {
        for m in ThemeMode::ALL {
            assert_eq!(ThemeMode::parse(m.key()), Some(m));
        }
        assert_eq!(ThemeMode::parse("nonsense"), None);
        for k in [ThemeKind::Dark, ThemeKind::Light, ThemeKind::HighContrast] {
            assert_eq!(ThemeKind::parse(k.key()), Some(k));
        }
        assert_eq!(parse_time("7:05"), Some(t(7, 5)));
        assert_eq!(parse_time("25:00"), None);
        assert_eq!(format_time(t(7, 5)), "07:05");
    }

    #[test]
    fn legacy_bool_maps_to_modes() {
        assert_eq!(ThemeMode::from_legacy_is_dark(true), ThemeMode::Dark);
        assert_eq!(ThemeMode::from_legacy_is_dark(false), ThemeMode::Light);
    }
}
