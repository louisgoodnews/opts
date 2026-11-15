//! Parts of a text field: the editable input, the placeholder and the clear button.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::text::{EditableText, LineBreak, TextCursorStyle, TextLayout};
use bevy::ui_widgets::TextInput;
use bevy::window::SystemCursorIcon;

use crate::{
    icons,
    interaction::{focus::Focusable, tooltip::Tooltip},
    l10n::L10nTooltip,
    theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale},
    tokens::components::{BackgroundRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

use super::super::super::button::Button;
use super::super::{TextFieldClear, TextFieldInput, TextFieldPlaceholder};
use super::TextFieldBuilder;

impl TextFieldBuilder {
    pub(super) fn spawn_input_entity(&self, commands: &mut Commands, root: Entity, input: Entity) {
        commands.entity(input).insert((
            Node {
                flex_grow: 1.0,
                min_width: Val::Px(40.0),
                overflow: Overflow::clip(),
                ..default()
            },
            TextInput,
            EditableText {
                max_characters: self.max_characters,
                ..EditableText::new(&self.initial_text)
            },
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextColor::default(),
            ForegroundRole(ColorRole::OnSurface),
            TypeStyleToken(TypeScale::BodyMedium),
            TextCursorStyle::default(),
            EntityCursor::System(SystemCursorIcon::Text),
            Focusable::default(),
            TabIndex(0),
            TextFieldInput(root),
            Name::new("TextFieldInput"),
        ));
    }

    pub(super) fn spawn_placeholder_entity(
        &self,
        commands: &mut Commands,
        root: Entity,
        placeholder: Entity,
        has_text: bool,
    ) {
        commands.entity(placeholder).insert((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(if self.compact { 10.0 } else { 16.0 }),
                display: if has_text {
                    Display::None
                } else {
                    Display::Flex
                },
                ..default()
            },
            Text::new(self.placeholder.clone()),
            TextColor::default(),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            TypeStyleToken(TypeScale::BodyMedium),
            Pickable::IGNORE,
            TextFieldPlaceholder(root),
        ));
    }

    pub(super) fn spawn_clear_button(
        &self,
        commands: &mut Commands,
        root: Entity,
        clear: Entity,
        has_text: bool,
        bg_role: ColorRole,
    ) {
        commands.entity(clear).insert((
            Button,
            Node {
                display: if has_text {
                    Display::Flex
                } else {
                    Display::None
                },
                min_width: Val::Px(32.0),
                min_height: Val::Px(32.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundRole(bg_role),
            RadiusToken(RadiusLevel::Full),
            Tooltip::new(""),
            L10nTooltip(crate::l10n::keys::CLEAR),
            TextFieldClear(root),
            Name::new("TextFieldClear"),
        ));
        commands.entity(clear).with_children(|p| {
            p.spawn((
                icons::icon_text(icons::CLOSE, 16.0, TextColor::default().0),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        });
    }
}
