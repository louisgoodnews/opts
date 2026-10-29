//! Settings whose value `0` means "chosen for this computer": the row shows what that choice is
//! now (M-0080), not a bare `0`.

use opts_gui::l10n::Localization;
use opts_hardware::recommend::MapStats;
use opts_hardware::HardwareProfile;

/// The settings with an automatic zero and the message that names the choice (variable `radius`).
const AUTOMATIC: &[(&str, &str)] = &[("gameplay.tile_radius", "set-tile-radius-auto")];

/// The text a stepper shows for `key` at value `cur`: the value, or for an automatic zero the
/// value the computer picks for a typical map.
pub fn shown_value(
    loc: &Localization,
    hardware: Option<&HardwareProfile>,
    key: &str,
    cur: &str,
) -> String {
    let Some((_, message)) = AUTOMATIC.iter().find(|(k, _)| *k == key) else {
        return cur.to_string();
    };
    if cur != "0" {
        return cur.to_string();
    }
    let radius = crate::in_game::tile_loading::recommended(hardware, &MapStats::default());
    loc.t_args(message, &[("radius".to_string(), (radius as i32).into())])
}

/// The stepper's value text of an integer setting.
pub fn value_text(
    c: &mut bevy::prelude::Commands,
    group: bevy::prelude::Entity,
    ctx: &super::Ctx,
    d: &crate::settings_registry::SettingDescriptor,
    cur: &str,
) {
    let text = shown_value(ctx.loc, ctx.hardware, d.key, cur);
    super::value_text(c, group, d.key, text);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> Localization {
        let mut loc = Localization::new("en");
        loc.extend(
            "en",
            include_str!("../../settings_registry/messages_en.ftl"),
        );
        loc.extend(
            "de",
            include_str!("../../settings_registry/messages_de.ftl"),
        );
        loc
    }

    #[test]
    fn only_an_automatic_zero_is_replaced() {
        let loc = loc();
        let text = |key, cur| shown_value(&loc, None, key, cur);
        assert!(text("gameplay.tile_radius", "0").starts_with("Automatic (currently "));
        assert_eq!(text("gameplay.tile_radius", "3"), "3");
        assert_eq!(text("gameplay.traffic", "0"), "0");
    }

    #[test]
    fn the_message_exists_in_german_too() {
        assert!(loc().has("de", "set-tile-radius-auto"));
    }
}
