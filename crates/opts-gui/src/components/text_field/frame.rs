//! The frame states of the text field: disabled treatment and the caret colours.

use bevy::prelude::*;
use bevy::text::TextCursorStyle;
use bevy::ui::InteractionDisabled;

use crate::interaction::{layer::FocusProxy, state::{Disabled, DisabledReason}};
use crate::theme::{color::ColorRole, theme::MaterialTheme};

use super::{M3TextField, TextFieldInput};

/// Query item of [`text_field_disabled_system`].
type DisabledFrameItem<'a> = (&'a FocusProxy, Option<&'a DisabledReason>);

/// A disabled frame makes its input read-only and unfocusable (unless it has a reason).
pub fn text_field_disabled_system(
    mut commands: Commands,
    added: Query<DisabledFrameItem, (With<M3TextField>, Added<Disabled>)>,
    mut removed: RemovedComponents<Disabled>,
    frames: Query<&FocusProxy, With<M3TextField>>,
) {
    for (proxy, reason) in &added {
        let mut input = commands.entity(proxy.0);
        input.try_insert((Disabled, InteractionDisabled));
        if let Some(r) = reason {
            input.try_insert(r.clone());
        }
    }
    for frame in removed.read() {
        if let Ok(proxy) = frames.get(frame) {
            commands
                .entity(proxy.0)
                .try_remove::<(Disabled, InteractionDisabled, DisabledReason)>();
        }
    }
}

/// Caret and selection colours follow the theme.
pub fn text_field_cursor_style_system(
    theme: Res<MaterialTheme>,
    mut inputs: Query<&mut TextCursorStyle, With<TextFieldInput>>,
) {
    for mut style in &mut inputs {
        let caret = theme.scheme.get(ColorRole::Primary);
        let selection = theme.scheme.get(ColorRole::Primary).with_alpha(0.35);
        if style.color != caret {
            style.color = caret;
            style.selection_color = selection;
            style.unfocused_selection_color = selection.with_alpha(0.15);
        }
    }
}
