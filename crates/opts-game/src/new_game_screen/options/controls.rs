//! The control of one equipment option: a switch, a choice box or a number field, in its line.

use super::*;

/// The control that fits `item`: a switch, a choice box or a number field.
pub(super) fn option_control(commands: &mut Commands, item: &SetvarItem) -> Entity {
    let id = format!("wizard.what.option.{}", opts_gui::ui_id::slug(&item.name));
    match item.kind {
        SetvarKind::Choice if !item.values.is_empty() => {
            let at = item
                .values
                .iter()
                .position(|v| *v == item.value)
                .unwrap_or(0);
            let combo = ComboboxBuilder::new(item.label.clone())
                .with_options(item.values.iter().map(i32::to_string).collect())
                .with_selected(at)
                .spawn(commands);
            commands.entity(combo).insert((
                OptionChoice {
                    name: item.name.clone(),
                    default: item.default_value,
                    values: item.values.clone(),
                },
                UiId::new(id),
            ));
            combo
        }
        SetvarKind::Number | SetvarKind::Choice => {
            let field = TextFieldBuilder::outlined(item.label.clone())
                .with_text(item.value.to_string())
                .with_max_characters(8)
                .spawn(commands);
            commands.entity(field).insert((
                OptionNumber {
                    name: item.name.clone(),
                    default: item.default_value,
                    min: item.min_val,
                    max: item.max_val,
                },
                UiId::new(id),
            ));
            field
        }
        SetvarKind::Switch | SetvarKind::Fraction => {
            let switch = SwitchBuilder::new(item.label.clone())
                .with_value(item.value != 0)
                .spawn(commands);
            commands.entity(switch).insert((
                OptionSwitch {
                    name: item.name.clone(),
                    default: item.default_value,
                },
                UiId::new(id),
            ));
            switch
        }
    }
}

/// One option: a line with its control and, for a choice box or number field, its name.
pub(super) fn option_line(commands: &mut Commands, item: &SetvarItem) -> Entity {
    // A row never shrinks below its control (the list scrolls instead), and a choice box or a
    // number field gets its name as a label beside it: the box shows only the value.
    let line = commands
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            min_height: Val::Px(56.0),
            flex_shrink: 0.0,
            ..row(12.0)
        })
        .id();
    let control = option_control(commands, item);
    if !matches!(item.kind, SetvarKind::Switch | SetvarKind::Fraction) {
        let label = commands
            .spawn((
                plain(
                    item.label.clone(),
                    TypeScale::BodyMedium,
                    ColorRole::OnSurface,
                ),
                Node {
                    flex_grow: 1.0,
                    flex_shrink: 1.0,
                    ..default()
                },
            ))
            .id();
        commands.entity(line).add_child(label);
    }
    commands.entity(line).add_child(control);
    line
}
