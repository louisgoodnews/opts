//! The setting controls that choose a value: switch, option buttons and stepper with presets.

use super::*;

/// A switch.
pub(super) fn bool_control(
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
pub(super) fn enum_control(
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
pub(super) fn stepper_control(
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
    super::super::auto::value_text(c, group, ctx, d, cur);
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
