//! The `[-] value [+]` steppers: the label, the value box and the two step buttons.

use bevy::prelude::*;
use opts_gui::icons::icon_text;
use opts_gui::l10n::L10nKey;
use opts_gui::patterns::glyphs;
use opts_gui::prelude::*;
use opts_gui::ui_tree::A11yHidden;

use super::field::Field;

pub(super) fn glyph(code: &'static str, size: f32, role: ColorRole) -> impl Bundle {
    (
        icon_text(code, size, TextColor::default().0),
        ForegroundRole(role),
        A11yHidden,
        Pickable::IGNORE,
    )
}

fn step_button(commands: &mut Commands, id: String, delta: i32, action: impl Bundle) -> Entity {
    let button = ButtonBuilder::text(String::new()).spawn(commands);
    commands.entity(button).insert((
        action,
        UiId::new(id),
        Tooltip::new(""),
        opts_gui::l10n::L10nTooltip(if delta < 0 {
            "wiz-step-less"
        } else {
            "wiz-step-more"
        }),
        Node {
            width: Val::Px(40.0),
            height: Val::Px(40.0),
            min_width: Val::Px(40.0),
            min_height: Val::Px(40.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
    ));
    let g = commands
        .spawn(glyph(
            if delta < 0 {
                glyphs::REMOVE
            } else {
                glyphs::ADD
            },
            20.0,
            ColorRole::OnSurfaceVariant,
        ))
        .id();
    commands.entity(button).add_child(g);
    button
}

/// One stepper: label above, `[-] value [+]` below. The buttons get the ids
/// `<id_prefix>.<field>.minus|plus` and the bundle `action(delta)`; the value text gets `marker`
/// (the owner keeps it current).
pub fn spawn_stepper<B: Bundle>(
    commands: &mut Commands,
    field: Field,
    value: String,
    marker: impl Bundle,
    id_prefix: &str,
    action: impl Fn(i32) -> B,
) -> Entity {
    let wrap = commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            min_width: Val::Px(0.0),
            min_height: Val::Px(0.0),
            ..default()
        })
        .id();
    let label = commands
        .spawn((
            Text::new(field.label_key()),
            L10nKey(field.label_key()),
            TextColor::default(),
            TypeStyleToken(TypeScale::Label),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ))
        .id();
    let line = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let minus = step_button(
        commands,
        format!("{id_prefix}.{}.minus", field.id()),
        -1,
        action(-1),
    );
    let val = commands
        .spawn((
            Text::new(value),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleMedium),
            ForegroundRole(ColorRole::OnSurface),
            marker,
            TextLayout::new(Justify::Center, LineBreak::NoWrap),
            Node {
                width: Val::Px(field.value_width()),
                justify_content: JustifyContent::Center,
                ..default()
            },
        ))
        .id();
    let plus = step_button(
        commands,
        format!("{id_prefix}.{}.plus", field.id()),
        1,
        action(1),
    );
    commands.entity(line).add_children(&[minus, val, plus]);
    commands.entity(wrap).add_children(&[label, line]);
    wrap
}

/// A wrapping row of steppers.
pub fn spawn_stepper_row(commands: &mut Commands, steppers: &[Entity]) -> Entity {
    let row = commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(16.0),
            row_gap: Val::Px(8.0),
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            min_width: Val::Px(0.0),
            ..default()
        })
        .id();
    commands.entity(row).add_children(steppers);
    row
}
