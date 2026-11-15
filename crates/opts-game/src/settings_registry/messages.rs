//! The settings messages: the localization that knows them.

use opts_gui::l10n::Localization;

/// A [`Localization`] for `locale` that also knows the messages of the settings (tests and the
/// generated reference).
#[must_use]
pub fn localization(locale: &str) -> Localization {
    let mut loc = Localization::new(locale);
    add_messages(&mut loc);
    loc
}

/// Adds the settings messages (`en`, `de`) to `loc`.
pub fn add_messages(loc: &mut Localization) {
    loc.extend("en", include_str!("messages_en.ftl"));
    loc.extend("en", include_str!("messages_lighting_en.ftl"));
    loc.extend("en", include_str!("messages_keys_en.ftl"));
    loc.extend("en", include_str!("messages_vehicle_en.ftl"));
    loc.extend("en", include_str!("messages_actions_en.ftl"));
    loc.extend("de", include_str!("messages_de.ftl"));
    loc.extend("de", include_str!("messages_lighting_de.ftl"));
    loc.extend("de", include_str!("messages_keys_de.ftl"));
    loc.extend("de", include_str!("messages_vehicle_de.ftl"));
    loc.extend("de", include_str!("messages_actions_de.ftl"));
}
