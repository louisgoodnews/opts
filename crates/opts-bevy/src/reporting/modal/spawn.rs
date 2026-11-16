//! Spawns the report modal: the screenshot preview, the title and text area, and the buttons.

use super::*;

fn spawn_preview_frame(p: &mut ChildSpawnerCommands, preview: Option<Handle<Image>>, aspect: f32) {
    p.spawn(Node {
        height: Val::Vh(25.0),
        width: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|frame| {
        if let Some(handle) = preview {
            frame.spawn((
                ImageNode::new(handle),
                Node {
                    height: Val::Vh(25.0),
                    width: Val::Vh(25.0 * aspect),
                    max_width: Val::Percent(100.0),
                    ..default()
                },
            ));
        }
    });
}

fn spawn_dialog_buttons(p: &mut ChildSpawnerCommands, loc: Option<&Localization>) {
    p.spawn(Node {
        column_gap: Val::Px(12.0),
        justify_content: JustifyContent::FlexEnd,
        flex_shrink: 0.0,
        ..default()
    })
    .with_children(|row| {
        let report = ButtonBuilder::filled(tr(loc, keys::SAVE, &[]))
            .with_key(keys::SAVE.0)
            .spawn_child(row);
        let cancel = ButtonBuilder::outlined(tr(loc, keys::CANCEL, &[]))
            .with_key(keys::CANCEL.0)
            .spawn_child(row);
        row.commands().queue(move |world: &mut World| {
            world.entity_mut(report).insert(ReportButton);
            world.entity_mut(cancel).insert(CancelButton);
        });
    });
}

/// Spawns the modal: screenshot preview, label, text area, Report/Cancel.
pub fn spawn_modal(
    mut commands: Commands,
    data: Res<ReportModalData>,
    images: Res<Assets<Image>>,
    loc: Option<Res<Localization>>,
) {
    let loc = loc.as_deref();
    let (root, content) = ModalBuilder::new().spawn(&mut commands);
    commands.entity(root).insert(ReportModalRoot);
    let preview = data.screenshot.clone();
    let aspect = preview
        .as_ref()
        .and_then(|h| images.get(h))
        .map(|i| i.width() as f32 / i.height().max(1) as f32)
        .unwrap_or(16.0 / 9.0);
    let mut text_area = Entity::PLACEHOLDER;
    commands.entity(content).with_children(|p| {
        p.spawn((
            Text::new(tr(loc, keys::TITLE, &[])),
            L10nKey(keys::TITLE.0),
            TextColor::default(),
            TypeStyleToken(TypeScale::TitleLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        spawn_preview_frame(p, preview, aspect);
        p.spawn((
            Text::new(tr(loc, keys::PREVIEW, &[])),
            L10nKey(keys::PREVIEW.0),
            TextColor::default(),
            TypeStyleToken(TypeScale::Label),
            ForegroundRole(ColorRole::OnSurfaceVariant),
        ));
        p.spawn((
            Text::new(tr(loc, keys::DESCRIBE, &[])),
            L10nKey(keys::DESCRIBE.0),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyLarge),
            ForegroundRole(ColorRole::OnSurface),
        ));
        text_area = TextAreaBuilder::new()
            .with_placeholder(tr(loc, keys::PLACEHOLDER, &[]))
            .autofocus(true)
            .spawn_child(p);
        p.spawn((
            Text::new(String::new()),
            TextColor::default(),
            TypeStyleToken(TypeScale::BodyMedium),
            ForegroundRole(ColorRole::Error),
            ReportErrorText,
        ));
        spawn_dialog_buttons(p, loc);
    });
    // Focus the text area so the user can type immediately (clears focus elsewhere).
    commands.queue(move |world: &mut World| {
        world.write_message(FocusEvent::Set(text_area));
    });
}
