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

/// A plain value text (a number, a time, a path).
fn value_text(c: &mut Commands, parent: Entity, key: &'static str, text: String) -> Entity {
    c.spawn((
        SettingValueText(key),
        Text::new(text),
        TextColor::default(),
        TypeStyleToken(TypeScale::BodyMedium),
        ForegroundRole(ColorRole::OnSurface),
        Node {
            min_width: Val::Px(48.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        ChildOf(parent),
    ))
    .id()
}

/// How a control is shown: usable, or why not.
#[derive(Clone)]
enum Usable {
    Yes,
    NotYet(&'static opts_gui::not_yet::NotYetPlace),
    Needs(String),
}

fn usable(ctx: &Ctx, d: &SettingDescriptor) -> Usable {
    if ctx.light_locked(d.key) {
        let msg = if d.key == crate::settings_registry::lighting::TARGET_FPS_KEY {
            "set-light-governor-disabled"
        } else {
            "set-light-auto-disabled"
        };
        return Usable::Needs(ctx.loc.t(msg));
    }
    match d.available {
        Available::Always => Usable::Yes,
        Available::NotYet(p) => Usable::NotYet(p),
        Available::Capability(name) if ctx.caps.has(name) => Usable::Yes,
        Available::Capability(name) => Usable::Needs(
            ctx.loc
                .t_args("settings-needs", &[("what".into(), name.into())]),
        ),
    }
}

fn mark(c: &mut Commands, e: Entity, u: &Usable) {
    match u {
        Usable::Yes => {}
        Usable::NotYet(p) => {
            c.entity(e).insert(NotAvailableYet::new(p).without_label());
        }
        Usable::Needs(reason) => {
            c.entity(e)
                .insert((Disabled, DisabledReason(reason.clone().into())));
        }
    }
}

/// A button of a row (`role` is the last part of its id).
#[allow(clippy::too_many_arguments)]
fn button(
    c: &mut Commands,
    parent: Entity,
    d: &SettingDescriptor,
    role: String,
    label_key: Option<&'static str>,
    text: &str,
    selected: bool,
    op: SettingOp,
    u: &Usable,
) -> Entity {
    let mut b = ButtonBuilder::outlined("");
    b.label = label_key.unwrap_or(text).to_string();
    if let Some(k) = label_key {
        b = b.with_key(k);
    }
    let e = b.spawn(c);
    c.entity(e).insert((
        op,
        UiId::new(format!("settings/{}/{}/{role}", d.category.id(), d.key)),
        ChildOf(parent),
    ));
    if selected {
        c.entity(e).insert((
            BackgroundRole(ROLES_SELECTED.0),
            BorderRole(ROLES_SELECTED.2),
        ));
        c.queue(move |world: &mut World| {
            let kids: Vec<Entity> = world
                .get::<Children>(e)
                .map(|k| k.iter().collect())
                .unwrap_or_default();
            for k in kids {
                if let Some(mut fg) = world.get_mut::<ForegroundRole>(k) {
                    fg.0 = ROLES_SELECTED.1;
                }
            }
        });
    }
    mark(c, e, u);
    e
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

/// A small localized note with variables below a language.
fn note(c: &mut Commands, parent: Entity, key: &'static str, args: Args, role: ColorRole) {
    c.spawn((
        Text::new(key),
        L10nKey(key),
        L10nArgs(args),
        TextColor::default(),
        TypeStyleToken(TypeScale::Label),
        ForegroundRole(role),
        Node {
            max_width: Val::Px(280.0),
            ..default()
        },
        ChildOf(parent),
    ));
}

mod language;

mod auto;
mod controls;
pub use auto::shown_value;
pub use controls::spawn_control;

/// The title, description and notes of a row, in a column that takes the room the controls leave.
fn spawn_texts(c: &mut Commands, row: Entity, d: &SettingDescriptor) {
    let text = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(220.0),
                min_width: Val::Px(0.0),
                row_gap: Val::Px(2.0),
                ..default()
            },
            ChildOf(row),
        ))
        .id();
    label(
        c,
        text,
        d.label_key,
        TypeScale::TitleMedium,
        ColorRole::OnSurface,
    );
    if d.is_stored() || matches!(d.kind, SettingKind::Action) {
        label(
            c,
            text,
            d.description_key,
            TypeScale::BodyMedium,
            ColorRole::OnSurfaceVariant,
        );
    }
    if matches!(d.available, Available::NotYet(_)) {
        c.entity(text).with_children(|p| {
            spawn_label(p);
        });
    }
    if matches!(d.apply, crate::settings_registry::Apply::OnRestart) {
        label(
            c,
            text,
            "settings-restart-note",
            TypeScale::Label,
            ColorRole::OnSurfaceVariant,
        );
    }
}

/// The *Reset* button of a stored setting (disabled with the reason while it is at its default).
fn spawn_reset(c: &mut Commands, right: Entity, ctx: &Ctx, d: &SettingDescriptor, cur: &str) {
    if d.is_stored() && !matches!(d.available, Available::NotYet(_)) && !ctx.light_locked(d.key) {
        let at_default = cur == d.default;
        let b = ButtonBuilder::text("settings-reset")
            .with_key("settings-reset")
            .with_disabled(at_default)
            .spawn(c);
        c.entity(b).insert((
            SettingOp::Reset { key: d.key },
            UiId::new(format!("settings/{}/{}/reset", d.category.id(), d.key)),
            ChildOf(right),
        ));
        if at_default {
            c.entity(b)
                .insert(DisabledReason(ctx.loc.t("settings-reset-disabled").into()));
        }
    }
}

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
