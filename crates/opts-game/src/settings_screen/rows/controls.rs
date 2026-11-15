//! The control group of a setting row: switch, chips, stepper, key and path controls.

use super::language::language_list;
use super::*;

mod choice;
mod entry;

use choice::{bool_control, enum_control, stepper_control};
use entry::{action_control, binding_control, path_control, text_control};

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
