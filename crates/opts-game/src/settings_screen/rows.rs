//! One row of the settings page and the control of each setting kind.

use bevy::prelude::*;
use opts_gui::components::button::ButtonBuilder;
use opts_gui::components::switch::SwitchBuilder;
use opts_gui::interaction::state::{Disabled, DisabledReason};
use opts_gui::l10n::{L10nKey, Localization};
use opts_gui::not_yet::{spawn_label, NotAvailableYet};
use opts_gui::prelude::*;

use crate::settings::GameSettings;
use crate::settings_registry::{
    Available, SettingDescriptor, SettingKind, SettingsCapabilities, SettingsRegistry,
};

use super::input::{SettingOp, SettingSwitch};

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

/// The control group of a setting, below `parent`.
pub fn spawn_control(
    c: &mut Commands,
    parent: Entity,
    ctx: &Ctx,
    d: &SettingDescriptor,
    cur: &str,
) {
    let u = usable(ctx, d);
    let group = c
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                row_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(parent),
        ))
        .id();
    match d.kind {
        SettingKind::Bool => {
            let e = SwitchBuilder::standalone()
                .with_value(cur == "true")
                .spawn(c);
            c.entity(e).insert((
                SettingSwitch(d.key),
                UiId::new(format!("settings/{}/{}/switch", d.category.id(), d.key)),
                ChildOf(group),
            ));
            mark(c, e, &u);
        }
        SettingKind::Enum(options) => {
            for o in options {
                let key = d.key;
                button(
                    c,
                    group,
                    d,
                    format!("option-{}", o.value),
                    Some(o.label_key),
                    "",
                    o.value == cur,
                    SettingOp::Set {
                        key,
                        value: o.value.to_string(),
                    },
                    &u,
                );
            }
        }
        SettingKind::Int { .. } | SettingKind::Float { .. } | SettingKind::Time => {
            button(
                c,
                group,
                d,
                "down".into(),
                Some("settings-step-down"),
                "",
                false,
                SettingOp::Step {
                    key: d.key,
                    dir: -1,
                },
                &u,
            );
            value_text(c, group, d.key, cur.to_string());
            button(
                c,
                group,
                d,
                "up".into(),
                Some("settings-step-up"),
                "",
                false,
                SettingOp::Step { key: d.key, dir: 1 },
                &u,
            );
        }
        SettingKind::Path | SettingKind::Binding => {
            if cur.is_empty() {
                label(
                    c,
                    group,
                    "settings-path-none",
                    TypeScale::BodyMedium,
                    ColorRole::OnSurfaceVariant,
                );
            } else {
                value_text(c, group, d.key, cur.to_string());
            }
            if matches!(d.kind, SettingKind::Path) {
                button(
                    c,
                    group,
                    d,
                    "choose".into(),
                    Some("settings-choose"),
                    "",
                    false,
                    SettingOp::Run { key: d.key },
                    &u,
                );
            }
        }
        SettingKind::Text => {
            if d.key == "theme_file" {
                let mut names = vec![String::new()];
                names.extend(crate::theme_files::discover_theme_files());
                for n in names {
                    let sel = n == cur;
                    if n.is_empty() {
                        button(
                            c,
                            group,
                            d,
                            "option-builtin".into(),
                            Some("settings-theme-file-builtin"),
                            "",
                            sel,
                            SettingOp::Set {
                                key: d.key,
                                value: n,
                            },
                            &u,
                        );
                    } else {
                        let id = format!("option-{}", opts_gui::ui_id::slug(&n));
                        button(
                            c,
                            group,
                            d,
                            id,
                            None,
                            &n.clone(),
                            sel,
                            SettingOp::Set {
                                key: d.key,
                                value: n,
                            },
                            &u,
                        );
                    }
                }
            } else {
                value_text(c, group, d.key, cur.to_string());
            }
        }
        SettingKind::Action => {
            button(
                c,
                group,
                d,
                "run".into(),
                Some(d.label_key),
                "",
                false,
                SettingOp::Run { key: d.key },
                &u,
            );
        }
    }
}

/// Spawns the row of `d` below `parent`.
pub fn spawn_row(c: &mut Commands, parent: Entity, ctx: &Ctx, d: &SettingDescriptor) {
    let cur = ctx.registry.text(ctx.settings, d.key).unwrap_or_default();
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
    if d.is_stored() && !matches!(d.available, Available::NotYet(_)) {
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
