//! Builder for constructing Material Design 3 text fields.

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::text::{EditableText, LineBreak, TextCursorStyle, TextLayout};
use bevy::ui_widgets::TextInput;
use bevy::window::SystemCursorIcon;

use crate::{
    icons,
    interaction::{
        focus::Focusable,
        layer::{FocusProxy, StateLayered},
        state::Hovered,
        tooltip::Tooltip,
    },
    l10n::L10nTooltip,
    theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

use super::super::button::Button;
use super::{
    M3TextField, TextFieldClear, TextFieldInput, TextFieldPlaceholder, TextFieldSync,
    TextFieldVariant,
};

/// Builder for constructing Material Design 3 text fields.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_input(mut commands: Commands) {
///     TextFieldBuilder::outlined("Route Code")
///         .with_placeholder("e.g. 100")
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TextFieldBuilder {
    pub label: String,
    pub placeholder: String,
    pub initial_text: String,
    pub variant: TextFieldVariant,
    pub compact: bool,
    pub max_characters: Option<usize>,
    pub full_width: bool,
    pub margin: UiRect,
}

impl TextFieldBuilder {
    fn new(label: impl Into<String>, variant: TextFieldVariant) -> Self {
        Self {
            label: label.into(),
            placeholder: String::new(),
            initial_text: String::new(),
            variant,
            compact: false,
            max_characters: None,
            full_width: false,
            margin: UiRect::DEFAULT,
        }
    }

    /// Creates an outlined text field builder with a label.
    pub fn outlined(label: impl Into<String>) -> Self {
        Self::new(label, TextFieldVariant::Outlined)
    }

    /// Creates a filled text field builder with a label.
    pub fn filled(label: impl Into<String>) -> Self {
        Self::new(label, TextFieldVariant::Filled)
    }

    /// Sets placeholder helper text displayed when input is empty.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the initial text content.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.initial_text = text.into();
        self
    }

    /// A denser frame (search bars, table filters); still at least 36 px high.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// The frame spans the full width of its parent.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Outer margin of the frame.
    pub fn with_margin(mut self, margin: UiRect) -> Self {
        self.margin = margin;
        self
    }

    /// Limits the number of characters.
    pub fn with_max_characters(mut self, max: usize) -> Self {
        self.max_characters = Some(max);
        self
    }

    fn frame_node(&self) -> Node {
        let (min_height, pad_y) = if self.compact {
            (36.0, 6.0)
        } else {
            (48.0, 12.0)
        };
        Node {
            width: if self.full_width {
                Val::Percent(100.0)
            } else {
                Val::Auto
            },
            margin: self.margin,
            min_height: Val::Px(min_height),
            min_width: Val::Px(if self.compact { 120.0 } else { 200.0 }),
            padding: UiRect::axes(
                Val::Px(if self.compact { 10.0 } else { 16.0 }),
                Val::Px(pad_y),
            ),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            border: if self.variant == TextFieldVariant::Outlined {
                UiRect::all(Val::Px(1.0))
            } else {
                UiRect {
                    bottom: Val::Px(2.0),
                    ..default()
                }
            },
            ..default()
        }
    }

    /// Spawns the text field using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let root = commands.spawn_empty().id();
        self.populate(commands, root);
        root
    }

    /// Spawns the text field as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let root = parent.spawn_empty().id();
        self.populate(parent.commands_mut(), root);
        root
    }

    fn spawn_input_entity(&self, commands: &mut Commands, root: Entity, input: Entity) {
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

    fn spawn_placeholder_entity(
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

    fn spawn_clear_button(
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

    fn populate(self, commands: &mut Commands, root: Entity) {
        let (bg_role, border_role) = match self.variant {
            TextFieldVariant::Outlined => (ColorRole::Surface, ColorRole::Outline),
            TextFieldVariant::Filled => (ColorRole::SurfaceContainerHighest, ColorRole::Primary),
        };
        let input = commands.spawn_empty().id();
        let placeholder = commands.spawn_empty().id();
        let clear = commands.spawn_empty().id();
        let has_text = !self.initial_text.is_empty();

        commands.entity(root).insert((
            self.frame_node(),
            M3TextField {
                variant: self.variant,
                label: self.label.clone(),
                text: self.initial_text.clone(),
                placeholder: self.placeholder.clone(),
                is_focused: false,
            },
            TextFieldSync(self.initial_text.clone()),
            BackgroundRole(bg_role),
            BorderRole(border_role),
            RadiusToken(RadiusLevel::ExtraSmall),
            Hovered::default(),
            StateLayered,
            FocusProxy(input),
            Name::new(format!("TextField {}", self.label)),
        ));
        self.spawn_input_entity(commands, root, input);
        self.spawn_placeholder_entity(commands, root, placeholder, has_text);
        self.spawn_clear_button(commands, root, clear, has_text, bg_role);
        commands
            .entity(root)
            .add_children(&[input, placeholder, clear]);
    }
}
