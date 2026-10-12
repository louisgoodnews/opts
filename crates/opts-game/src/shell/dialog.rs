//! Confirmation dialogs: one small modal that names the consequence and offers the choices.
//!
//! Used by *Quit OPTS* and *Leave drive…*. The first button in tree order gets the focus when the
//! dialog opens, so the order is *safe choice first*. Esc and the backdrop's cancel request
//! answer with the choice [`CANCEL`]; the dialog closes itself in every case.

use bevy::prelude::*;
use opts_gui::components::modal::{ModalEvent, ModalRoot};
use opts_gui::interaction::activation::Activated;
use opts_gui::l10n::{L10nArgs, L10nKey};
use opts_gui::prelude::*;

/// The choice reported when the dialog is cancelled with Esc.
pub const CANCEL: &str = "cancel";

/// How a dialog button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogButtonStyle {
    /// The safe, quiet choice (Cancel).
    Quiet,
    /// The expected choice.
    Primary,
    /// An irreversible choice (error colours).
    Destructive,
}

/// One button of a dialog.
#[derive(Debug, Clone, Copy)]
pub struct DialogButtonSpec {
    /// The choice id reported in [`DialogChoice`].
    pub choice: &'static str,
    /// Message key of the label.
    pub label_key: &'static str,
    /// Look.
    pub style: DialogButtonStyle,
}

/// What a dialog says and offers.
#[derive(Debug, Clone)]
pub struct DialogSpec {
    /// Identifies the dialog to the code that opened it (`shell.quit`).
    pub tag: &'static str,
    /// Message key of the title.
    pub title_key: &'static str,
    /// Message key of the body.
    pub body_key: &'static str,
    /// Variables of the body.
    pub body_args: L10nArgs,
    /// The buttons, safe choice first.
    pub buttons: Vec<DialogButtonSpec>,
    /// Choices that report [`DialogChoice`] but leave the dialog open (a folder chooser that
    /// has not answered yet).
    pub keep_open: Vec<&'static str>,
    /// An inline error under the body: message key and its variables.
    pub error: Option<(&'static str, L10nArgs)>,
}

/// Marker on the backdrop of a dialog.
#[derive(Component, Debug, Clone)]
pub struct ConfirmDialog {
    /// The dialog's tag.
    pub tag: &'static str,
    /// Choices that leave the dialog open.
    pub keep_open: Vec<&'static str>,
}

/// A button of a dialog.
#[derive(Component, Debug, Clone, Copy)]
pub struct DialogButton {
    /// The dialog backdrop.
    pub dialog: Entity,
    /// The choice id.
    pub choice: &'static str,
}

/// The user's answer; sent once, when the dialog closes.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogChoice {
    /// The dialog's tag.
    pub tag: &'static str,
    /// The chosen button's id, or [`CANCEL`].
    pub choice: &'static str,
}

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

/// Answers and closes a dialog when one of its buttons is activated.
pub fn dialog_button_system(
    mut commands: Commands,
    mut activated: MessageReader<Activated>,
    buttons: Query<&DialogButton>,
    dialogs: Query<&ConfirmDialog>,
    mut choices: MessageWriter<DialogChoice>,
) {
    for Activated(entity) in activated.read() {
        let Ok(button) = buttons.get(*entity) else {
            continue;
        };
        if let Ok(dialog) = dialogs.get(button.dialog) {
            choices.write(DialogChoice {
                tag: dialog.tag,
                choice: button.choice,
            });
            if !dialog.keep_open.contains(&button.choice) {
                commands.entity(button.dialog).try_despawn();
            }
        }
    }
}

/// Esc (the modal's cancel request) answers with [`CANCEL`] and closes the dialog.
pub fn dialog_cancel_system(
    mut commands: Commands,
    mut events: MessageReader<ModalEvent>,
    dialogs: Query<&ConfirmDialog, With<ModalRoot>>,
    mut choices: MessageWriter<DialogChoice>,
) {
    for ModalEvent::CancelRequested(modal) in events.read() {
        if let Ok(dialog) = dialogs.get(*modal) {
            choices.write(DialogChoice {
                tag: dialog.tag,
                choice: CANCEL,
            });
            commands.entity(*modal).try_despawn();
        }
    }
}

/// Registers the dialog message and systems.
pub fn build(app: &mut App) {
    app.add_message::<DialogChoice>()
        .add_systems(Update, (dialog_button_system, dialog_cancel_system));
}
