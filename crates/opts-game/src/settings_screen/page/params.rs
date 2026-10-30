//! System-parameter bundles of the settings page's rebuild and refresh systems.

use bevy::ecs::system::SystemParam;

use super::*;

/// What a rebuild of the page is worked out from.
#[derive(SystemParam)]
pub struct RebuildFacts<'w> {
    /// The theme.
    pub theme: Res<'w, MaterialTheme>,
    /// The descriptors.
    pub registry: Res<'w, SettingsRegistry>,
    /// The values.
    pub settings: Res<'w, GameSettings>,
    /// What the machine can do.
    pub caps: Res<'w, SettingsCapabilities>,
    /// The background look at the installation.
    pub facts: Res<'w, crate::content_facts::ContentFactsStore>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The detected hardware.
    pub hardware: Option<Res<'w, crate::hardware::HardwareState>>,
    /// The lighting quality.
    pub light: Option<Res<'w, opts_bevy::light_quality::LightQuality>>,
    /// The fonts to choose from.
    pub fonts: Option<Res<'w, crate::ui_typography::FontChoices>>,
}

/// What makes the page dirty when it changes.
#[derive(SystemParam)]
pub struct DirtyFacts<'w> {
    /// The descriptors.
    pub registry: Res<'w, SettingsRegistry>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The background look at the installation.
    pub facts: Res<'w, crate::content_facts::ContentFactsStore>,
    /// The lighting quality.
    pub light: Option<Res<'w, opts_bevy::light_quality::LightQuality>>,
    /// The fonts to choose from.
    pub fonts: Option<Res<'w, crate::ui_typography::FontChoices>>,
}

/// What a row refresh reads.
#[derive(SystemParam)]
pub struct RowFacts<'w> {
    /// The descriptors.
    pub registry: Res<'w, SettingsRegistry>,
    /// The values.
    pub settings: Res<'w, GameSettings>,
    /// The texts.
    pub loc: Res<'w, Localization>,
    /// The theme.
    pub theme: Res<'w, MaterialTheme>,
    /// The detected hardware.
    pub hardware: Option<Res<'w, crate::hardware::HardwareState>>,
}

/// Query of the elements `refresh_rows` updates.
pub type OptionButtons<'w, 's> = Query<
    'w,
    's,
    (
        &'static SettingOp,
        &'static mut BackgroundRole,
        &'static mut BorderRole,
        Option<&'static Children>,
    ),
>;

/// The nodes `refresh_rows` updates in place.
#[derive(SystemParam)]
pub struct RowNodes<'w, 's> {
    /// The option buttons.
    pub options: OptionButtons<'w, 's>,
    /// The labels (colour roles).
    pub labels: Query<'w, 's, &'static mut ForegroundRole>,
    /// The switches.
    pub switches: Query<'w, 's, (&'static SettingSwitch, &'static mut M3Switch)>,
    /// The value texts.
    pub values: Query<'w, 's, (&'static SettingValueText, &'static mut Text)>,
    /// The reset buttons.
    pub resets: Query<
        'w,
        's,
        (
            Entity,
            &'static SettingOp,
            Has<opts_gui::interaction::state::Disabled>,
        ),
    >,
    /// The "theme now" line.
    pub now: Query<'w, 's, &'static mut L10nArgs, With<ThemeNowText>>,
}
