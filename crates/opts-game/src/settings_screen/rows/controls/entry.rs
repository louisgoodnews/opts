//! The setting controls that take text, a key, a path or start an action.

use super::*;

/// The key of a binding with the button that waits for the next key.
pub(super) fn binding_control(
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
pub(super) fn path_control(
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

/// A theme file as one button per file, any other text as its value.
pub(super) fn text_control(
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
pub(super) fn action_control(
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
