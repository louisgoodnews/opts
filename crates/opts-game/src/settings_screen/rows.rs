//! One row of the settings page and the control of each setting kind.

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::switch::SwitchBuilder;
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::l10n::fluent::Args;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::not_yet::{spawn_label, NotAvailableYet};
use opts_gui::prelude::*;

use crate::settings::GameSettings;
use crate::settings_registry::{
    Available, SettingDescriptor, SettingKind, SettingsCapabilities, SettingsRegistry,
};

use super::input::{SettingCombo, SettingOp, SettingSwitch};
use opts_gui::components::combobox::ComboboxBuilder;

mod helpers;
mod texts;

use helpers::{button, mark, note, usable, value_text, Usable};
use texts::{spawn_reset, spawn_texts};

/// Everything a row needs to build itself.
pub struct Ctx<'a> {
    /// The descriptors.
    pub registry: &'a SettingsRegistry,
    /// The values.
    pub settings: &'a GameSettings,
    /// The messages.
    pub loc: &'a Localization,
    /// What the machine can do.
    pub caps: &'a SettingsCapabilities,
    /// The detected hardware, for the Advanced page.
    pub hardware: Option<&'a opts_hardware::HardwareProfile>,
    /// Folder of the stored hardware profile.
    pub hardware_file: Option<std::path::PathBuf>,
    /// Theme files found by the background facts (never read from disk while building).
    pub theme_files: &'a [String],
    /// Font families the font setting offers besides the default.
    pub fonts: &'a [String],
    /// The effective lighting quality (what the governor applies in automatic mode).
    pub light: Option<&'a opts_bevy::light_quality::LightQuality>,
    /// The key setting that waits for the next key press.
    pub rebinding: Option<&'a str>,
}

impl Ctx<'_> {
    /// Whether `key` is a lighting control that the current mode makes inert: a custom control in
    /// automatic mode, or the governor's target frame rate in custom mode.
    #[must_use]
    pub fn light_locked(&self, key: &str) -> bool {
        use crate::settings_registry::lighting as l;
        let automatic = l::is_automatic(self.settings);
        (l::is_custom_key(key) && automatic) || (key == l::TARGET_FPS_KEY && !automatic)
    }
}

/// A localized text element.
pub fn label(
    c: &mut Commands,
    parent: Entity,
    key: &'static str,
    style: TypeScale,
    role: ColorRole,
) -> Entity {
    c.spawn((
        Text::new(key),
        L10nKey(key),
        TextColor::default(),
        TypeStyleToken(style),
        ForegroundRole(role),
        ChildOf(parent),
    ))
    .id()
}

/// Background, label and border roles of the selected option of a group.
pub const ROLES_SELECTED: (ColorRole, ColorRole, ColorRole) =
    (ColorRole::Primary, ColorRole::OnPrimary, ColorRole::Primary);
/// The same for the other options.
pub const ROLES_PLAIN: (ColorRole, ColorRole, ColorRole) =
    (ColorRole::Surface, ColorRole::Primary, ColorRole::Outline);

/// The current value of a stepper or text value, kept current by the page.
#[derive(Component, Debug, Clone, Copy)]
pub struct SettingValueText(pub &'static str);

mod language;

mod auto;
mod controls;
pub use auto::shown_value;
pub use controls::spawn_control;

/// Spawns the row of `d` below `parent`.
pub fn spawn_row(c: &mut Commands, parent: Entity, ctx: &Ctx, d: &SettingDescriptor) {
    let mut cur = ctx.registry.text(ctx.settings, d.key).unwrap_or_default();
    if ctx.light_locked(d.key) {
        // Automatic mode: show what the governor applies, not the stored custom value.
        if let Some(shown) = ctx
            .light
            .and_then(|q| crate::settings_registry::lighting::text_for(q, d.key))
        {
            cur = shown;
        }
    }
    let row = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                column_gap: Val::Px(16.0),
                row_gap: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(4.0), Val::Px(10.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderRole(ColorRole::OutlineVariant),
            Name::new(d.key),
            ChildOf(parent),
        ))
        .id();
    spawn_texts(c, row, d);
    let right = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    spawn_control(c, right, ctx, d, &cur);
    spawn_reset(c, right, ctx, d, &cur);
}
