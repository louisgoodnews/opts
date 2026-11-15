//! The clock rule of *Auto*: the times the light theme starts and ends, and their text forms.

use chrono::NaiveTime;

use super::ThemeKind;

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
