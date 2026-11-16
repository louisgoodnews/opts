//! Opening a confirmation dialog: the card, its texts and buttons, and the destructive colours.

use super::*;

/// Opens a dialog and returns its backdrop.
pub fn spawn_confirm_dialog(commands: &mut Commands, spec: DialogSpec) -> Entity {
    let (root, content) = ModalBuilder::new()
        .with_size_percent(40.0, 40.0)
        .with_width_range(420.0, 560.0)
        .spawn(commands);
    commands.entity(root).insert(ConfirmDialog {
        tag: spec.tag,
        keep_open: spec.keep_open.clone(),
    });
    commands.queue(move |world: &mut World| {
        if let Some(mut node) = world.entity_mut(content).get_mut::<Node>() {
            node.height = Val::Auto;
            node.row_gap = Val::Px(16.0);
        }
    });
    commands.entity(content).with_children(|card| {
        card.spawn((
            Text::new(spec.title_key),
            L10nKey(spec.title_key),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        card.spawn((
            Text::new(spec.body_key),
            L10nKey(spec.body_key),
            spec.body_args,
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ));
        if let Some((key, args)) = spec.error.clone() {
            card.spawn((
                Text::new(key),
                L10nKey(key),
                args,
                TextColor::default(),
                TypeStyleToken(TypeScale::BodyMedium),
                ForegroundRole(ColorRole::Error),
                UiId::new(format!("dialog.{}.error", spec.tag)),
            ));
        }
        card.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            column_gap: Val::Px(12.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            for (index, button) in spec.buttons.iter().enumerate() {
                let builder = match button.style {
                    DialogButtonStyle::Quiet => ButtonBuilder::text(button.label_key),
                    DialogButtonStyle::Primary | DialogButtonStyle::Destructive => {
                        ButtonBuilder::filled(button.label_key)
                    }
                }
                .with_key(button.label_key);
                let entity = builder.spawn_child(row);
                let mut commands = row.commands();
                commands.entity(entity).insert((
                    DialogButton {
                        dialog: root,
                        choice: button.choice,
                    },
                    UiId::new(format!("dialog.{}.{}", spec.tag, button.choice)),
                ));
                if index == 0 {
                    commands.entity(entity).insert(DefaultFocus);
                }
                if button.style == DialogButtonStyle::Destructive {
                    recolor_destructive(&mut commands, entity);
                }
            }
        });
    });
    root
}

/// Error colours for a filled button and its label.
fn recolor_destructive(commands: &mut Commands, button: Entity) {
    commands.entity(button).insert((
        BackgroundRole(ColorRole::Error),
        StateLayerRole(ColorRole::OnError),
    ));
    commands.queue(move |world: &mut World| {
        let children: Vec<Entity> = world
            .get::<Children>(button)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        for child in children {
            if let Some(mut fg) = world.entity_mut(child).get_mut::<ForegroundRole>() {
                fg.0 = ColorRole::OnError;
            }
        }
    });
}
