//! Search over the registry: label, description, keywords and key, every word must match.

use super::{SettingDescriptor, SettingsRegistry};
use opts_gui::l10n::Localization;

/// The searchable text of a setting in the active language (lower case).
fn haystack(d: &SettingDescriptor, loc: &Localization) -> String {
    let mut s = String::new();
    s.push_str(&loc.t(d.label_key));
    s.push(' ');
    s.push_str(&loc.t(d.description_key));
    s.push(' ');
    s.push_str(&d.keywords.join(" "));
    s.push(' ');
    s.push_str(d.key);
    s.to_lowercase()
}

/// The settings matching `query`, in registry order. An empty query matches nothing (the page
/// shows the category instead).
#[must_use]
pub fn search<'a>(
    registry: &'a SettingsRegistry,
    loc: &Localization,
    query: &str,
) -> Vec<&'a SettingDescriptor> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    registry
        .all()
        .into_iter()
        .filter(|d| {
            let h = haystack(d, loc);
            words.iter().all(|w| h.contains(w.as_str()))
        })
        .collect()
}
