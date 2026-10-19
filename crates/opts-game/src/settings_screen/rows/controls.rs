//! The control group of a setting row: switch, chips, stepper, key and path controls.

use super::language::language_list;
use super::*;

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
                // A switch has no text of its own: the setting's title is its accessible name
                // (and its tooltip), kept in the current language by the library.
                opts_gui::interaction::tooltip::Tooltip::new(String::new()),
                opts_gui::l10n::L10nTooltip(d.label_key),
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
            // Preset chips: one click to a usual value; the one that matches is marked.
            for preset in d.presets {
                button(
                    c,
                    group,
                    d,
                    format!("preset-{preset}"),
                    None,
                    &format!("{preset}{}", d.preset_suffix),
                    *preset == cur,
                    SettingOp::Set {
                        key: d.key,
                        value: (*preset).to_string(),
                    },
                    &u,
                );
            }
        }
        SettingKind::Binding => {
            value_text(c, group, d.key, cur.to_string());
            let waiting = ctx.rebinding == Some(d.key);
            button(
                c,
                group,
                d,
                "change".into(),
                Some(if waiting {
                    "settings-rebind-wait"
                } else {
                    "settings-rebind-change"
                }),
                "",
                waiting,
                SettingOp::Run { key: d.key },
                &u,
            );
        }
        SettingKind::Path => {
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
        SettingKind::Text if d.key == crate::ui_typography::SETTING_FONT => {
            button(
                c,
                group,
                d,
                "option-default".into(),
                Some("settings-font-default"),
                "",
                cur.is_empty(),
                SettingOp::Set {
                    key: d.key,
                    value: String::new(),
                },
                &u,
            );
            for family in ctx.fonts {
                button(
                    c,
                    group,
                    d,
                    format!("option-{}", opts_gui::ui_id::slug(family)),
                    None,
                    family,
                    family == cur,
                    SettingOp::Set {
                        key: d.key,
                        value: family.clone(),
                    },
                    &u,
                );
            }
        }
        SettingKind::Text if d.key == crate::language_packs::LANGUAGE_KEY => {
            language_list(c, group, ctx, d, cur, &u);
        }
        SettingKind::Text => {
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
