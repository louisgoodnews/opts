//! The control group of a setting row: switch, chips, stepper, key and path controls.

use super::language::language_list;
use super::*;

/// A switch.
fn bool_control(
    c: &mut Commands,
    group: Entity,
    (_ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    let e = SwitchBuilder::standalone()
        .with_value(cur == "true")
        .spawn(c);
    c.entity(e).insert((
        SettingSwitch(d.key),
        UiId::new(format!("settings/{}/{}/switch", d.category.id(), d.key)),
        ChildOf(group),
        // A switch has no text of its own: the setting's title is its accessible name
        // (and its tooltip), kept in the current language by the library.
        opts_gui::interaction::tooltip::Tooltip::new(String::new()),
        opts_gui::l10n::L10nTooltip(d.label_key),
    ));
    mark(c, e, u);
}

/// One button per option.
fn enum_control(
    c: &mut Commands,
    group: Entity,
    (_ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    let SettingKind::Enum(options) = d.kind else {
        return;
    };
    for o in options {
        let key = d.key;
        button(
            c,
            group,
            d,
            format!("option-{}", o.value),
            (Some(o.label_key), ""),
            (
                o.value == cur,
                SettingOp::Set {
                    key,
                    value: o.value.to_string(),
                },
            ),
            u,
        );
    }
}

/// A stepper with its value and the preset chips.
fn stepper_control(
    c: &mut Commands,
    group: Entity,
    (ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    button(
        c,
        group,
        d,
        "down".into(),
        (Some("settings-step-down"), ""),
        (
            false,
            SettingOp::Step {
                key: d.key,
                dir: -1,
            },
        ),
        u,
    );
    super::auto::value_text(c, group, ctx, d, cur);
    button(
        c,
        group,
        d,
        "up".into(),
        (Some("settings-step-up"), ""),
        (false, SettingOp::Step { key: d.key, dir: 1 }),
        u,
    );
    // Preset chips: one click to a usual value; the one that matches is marked.
    for preset in d.presets {
        button(
            c,
            group,
            d,
            format!("preset-{preset}"),
            (None, &format!("{preset}{}", d.preset_suffix)),
            (
                *preset == cur,
                SettingOp::Set {
                    key: d.key,
                    value: (*preset).to_string(),
                },
            ),
            u,
        );
    }
}

/// The key of a binding with the button that waits for the next key.
fn binding_control(
    c: &mut Commands,
    group: Entity,
    (ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    value_text(c, group, d.key, cur.to_string());
    let waiting = ctx.rebinding == Some(d.key);
    button(
        c,
        group,
        d,
        "change".into(),
        (
            Some(if waiting {
                "settings-rebind-wait"
            } else {
                "settings-rebind-change"
            }),
            "",
        ),
        (waiting, SettingOp::Run { key: d.key }),
        u,
    );
}

/// The path (or none) with the button that chooses it.
fn path_control(
    c: &mut Commands,
    group: Entity,
    (_ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
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
            (Some("settings-choose"), ""),
            (false, SettingOp::Run { key: d.key }),
            u,
        );
    }
}

/// Every installed family is a choice: a combo box with its search field.
fn font_control(
    c: &mut Commands,
    group: Entity,
    (ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    // Every installed family is a choice: a combo box with its search field.
    let mut values = vec![String::new()];
    values.extend(ctx.fonts.iter().cloned());
    if !cur.is_empty() && !values.iter().any(|v| v == cur) {
        values.push(cur.to_owned());
    }
    let labels: Vec<String> = values
        .iter()
        .map(|v| {
            if v.is_empty() {
                ctx.loc.t("settings-font-default")
            } else {
                v.clone()
            }
        })
        .collect();
    let selected = values.iter().position(|v| v == cur).unwrap_or(0);
    let combo = ComboboxBuilder::new(ctx.loc.t(d.label_key))
        .with_options(labels)
        .with_selected(selected)
        .spawn(c);
    c.entity(combo).insert((
        SettingCombo { key: d.key, values },
        UiId::new(format!("settings/{}/{}/combo", d.category.id(), d.key)),
        ChildOf(group),
    ));
    mark(c, combo, u);
}

/// A theme file as one button per file, any other text as its value.
fn text_control(
    c: &mut Commands,
    group: Entity,
    (ctx, d, cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    if d.key == "theme_file" {
        let mut names = vec![String::new()];
        names.extend(ctx.theme_files.iter().cloned());
        for n in names {
            let sel = n == cur;
            if n.is_empty() {
                button(
                    c,
                    group,
                    d,
                    "option-builtin".into(),
                    (Some("settings-theme-file-builtin"), ""),
                    (
                        sel,
                        SettingOp::Set {
                            key: d.key,
                            value: n,
                        },
                    ),
                    u,
                );
            } else {
                let id = format!("option-{}", opts_gui::ui_id::slug(&n));
                button(
                    c,
                    group,
                    d,
                    id,
                    (None, &n.clone()),
                    (
                        sel,
                        SettingOp::Set {
                            key: d.key,
                            value: n,
                        },
                    ),
                    u,
                );
            }
        }
    } else {
        value_text(c, group, d.key, cur.to_string());
    }
}

/// A button that runs the action.
fn action_control(
    c: &mut Commands,
    group: Entity,
    (_ctx, d, _cur): (&Ctx, &SettingDescriptor, &str),
    u: &Usable,
) {
    button(
        c,
        group,
        d,
        "run".into(),
        (Some(d.label_key), ""),
        (false, SettingOp::Run { key: d.key }),
        u,
    );
}

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
        SettingKind::Bool => bool_control(c, group, (ctx, d, cur), &u),
        SettingKind::Enum(_) => enum_control(c, group, (ctx, d, cur), &u),
        SettingKind::Int { .. } | SettingKind::Float { .. } | SettingKind::Time => {
            stepper_control(c, group, (ctx, d, cur), &u)
        }
        SettingKind::Binding => binding_control(c, group, (ctx, d, cur), &u),
        SettingKind::Path => path_control(c, group, (ctx, d, cur), &u),
        SettingKind::Text if d.key == crate::ui_typography::SETTING_FONT => {
            font_control(c, group, (ctx, d, cur), &u)
        }
        SettingKind::Text if d.key == crate::language_packs::LANGUAGE_KEY => {
            language_list(c, group, ctx, d, cur, &u);
        }
        SettingKind::Text => text_control(c, group, (ctx, d, cur), &u),
        SettingKind::Action => action_control(c, group, (ctx, d, cur), &u),
    }
}
