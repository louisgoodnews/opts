//! The texts of a setting row (title, description and notes) and its *Reset* button.

use super::*;

/// The title, description and notes of a row, in a column that takes the room the controls leave.
pub(super) fn spawn_texts(c: &mut Commands, row: Entity, d: &SettingDescriptor) {
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
pub(super) fn spawn_reset(c: &mut Commands, right: Entity, ctx: &Ctx, d: &SettingDescriptor, cur: &str) {
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
