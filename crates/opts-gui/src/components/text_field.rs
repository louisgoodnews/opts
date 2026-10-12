//! Material Design 3 single-line text field on top of Bevy's `TextInput` / `EditableText`.
//!
//! Typing uses the platform keyboard layout and IME, with caret, selection, word movement,
//! copy / cut / paste (system clipboard) and select-all handled by Bevy
//! (`docs/src/architecture/ui-spikes.md`, spike 1). This module adds the M3 frame, a placeholder,
//! a clear button, the [`TextChanged`] message, the error / disabled treatment and a programmatic
//! `text` that works in both directions.
//!
//! Structure: a *frame* entity ([`M3TextField`], border, background, state layer, focus ring)
//! holding the *input* ([`TextFieldInput`], the focusable `TextInput`), the *placeholder* and the
//! *clear button*. Focus lives on the input; the frame shows it through [`FocusProxy`].

use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::text::{EditableText, LineBreak, TextCursorStyle, TextEdit, TextLayout};
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::TextInput;
use bevy::window::SystemCursorIcon;

use crate::{
    icons,
    interaction::{
        activation::Activated,
        events::TextChanged,
        focus::{FocusEvent, Focusable},
        layer::{FocusProxy, StateLayered},
        state::{Disabled, DisabledReason, Focused, Hovered},
        tooltip::Tooltip,
    },
    theme::{color::ColorRole, shape::RadiusLevel, theme::MaterialTheme, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

use super::button::Button;

/// Variants for Material Design 3 text fields.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFieldVariant {
    #[default]
    Outlined,
    Filled,
}

/// The frame of a text field. `text` is the value: write it to set the field, read it (or
/// listen to [`TextChanged`]) to get what the user typed.
#[derive(Component, Debug, Clone, Default)]
pub struct M3TextField {
    pub variant: TextFieldVariant,
    pub text: String,
    pub placeholder: String,
    /// Mirror of "the input holds focus" (maintained by the library).
    pub is_focused: bool,
}

/// The focusable `TextInput` inside a field; points back to its frame.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldInput(pub Entity);

/// The placeholder text of a field (shown while the value is empty).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldPlaceholder(pub Entity);

/// The clear (x) button of a field (shown while the value is not empty).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldClear(pub Entity);

/// The text last exchanged between frame and input (decides which side changed).
#[derive(Component, Debug, Clone, Default)]
pub struct TextFieldSync(pub String);

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
            TextColor(Color::WHITE),
            ForegroundRole(ColorRole::OnSurface),
            TypeStyleToken(TypeScale::BodyMedium),
            TextCursorStyle::default(),
            EntityCursor::System(SystemCursorIcon::Text),
            Focusable::default(),
            TabIndex(0),
            TextFieldInput(root),
            Name::new("TextFieldInput"),
        ));
        commands.entity(placeholder).insert((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                display: if has_text {
                    Display::None
                } else {
                    Display::Flex
                },
                ..default()
            },
            Text::new(self.placeholder),
            TextColor(Color::WHITE),
            ForegroundRole(ColorRole::OnSurfaceVariant),
            TypeStyleToken(TypeScale::BodyMedium),
            Pickable::IGNORE,
            TextFieldPlaceholder(root),
        ));
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
            Tooltip::new("Clear"),
            TextFieldClear(root),
            Name::new("TextFieldClear"),
        ));
        commands.entity(clear).with_children(|p| {
            p.spawn((
                icons::icon_text(icons::CLOSE, 16.0, Color::WHITE),
                ForegroundRole(ColorRole::OnSurfaceVariant),
            ));
        });
        // Order: input, (placeholder overlays it), clear.
        commands
            .entity(root)
            .add_children(&[input, placeholder, clear]);
    }
}

/// Carries the programmatic `text` both ways and sends [`TextChanged`].
pub fn text_field_sync_system(
    mut fields: Query<(Entity, &mut M3TextField, &mut TextFieldSync)>,
    mut inputs: Query<(&TextFieldInput, &mut EditableText)>,
    mut events: MessageWriter<TextChanged>,
) {
    for (input, mut editable) in &mut inputs {
        let Ok((entity, mut field, mut sync)) = fields.get_mut(input.0) else {
            continue;
        };
        if field.text != sync.0 {
            // The owner wrote `text`: push it into the input.
            editable.editor_mut().set_text(&field.text);
            editable.queue_edit(TextEdit::TextEnd(false));
            sync.0.clone_from(&field.text);
            events.write(TextChanged {
                entity,
                text: field.text.clone(),
            });
            continue;
        }
        let typed = editable.value().to_string();
        if typed != sync.0 {
            sync.0.clone_from(&typed);
            field.text = typed.clone();
            events.write(TextChanged {
                entity,
                text: typed,
            });
        }
    }
}

/// Shows the placeholder only for an empty value and the clear button only for a non-empty one;
/// mirrors focus into [`M3TextField::is_focused`].
pub fn text_field_chrome_system(
    mut fields: Query<(&mut M3TextField, &FocusProxy)>,
    focused: Query<(), With<Focused>>,
    mut placeholders: Query<(&TextFieldPlaceholder, &mut Node, &mut Text), Without<TextFieldClear>>,
    mut clears: Query<(&TextFieldClear, &mut Node), Without<TextFieldPlaceholder>>,
) {
    for (mut field, proxy) in &mut fields {
        let has_focus = focused.contains(proxy.0);
        if field.is_focused != has_focus {
            field.is_focused = has_focus;
        }
    }
    let fields = fields.as_readonly();
    for (placeholder, mut node, mut text) in &mut placeholders {
        if let Ok((field, _)) = fields.get(placeholder.0) {
            let display = if field.text.is_empty() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
            if text.0 != field.placeholder {
                text.0.clone_from(&field.placeholder);
            }
        }
    }
    for (clear, mut node) in &mut clears {
        if let Ok((field, _)) = fields.get(clear.0) {
            let display = if field.text.is_empty() {
                Display::None
            } else {
                Display::Flex
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}

/// The clear button empties the value and puts focus back into the input.
pub fn text_field_clear_system(
    mut activated: MessageReader<Activated>,
    clears: Query<&TextFieldClear>,
    mut fields: Query<(&mut M3TextField, &FocusProxy)>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(clear) = clears.get(*entity) {
            if let Ok((mut field, proxy)) = fields.get_mut(clear.0) {
                field.text.clear();
                focus.write(FocusEvent::Set(proxy.0));
            }
        }
    }
}

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

/// Plugin registering TextField widget systems.
#[derive(Default)]
pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TextChanged>().add_message::<Activated>();
        app.add_systems(
            Update,
            (
                text_field_sync_system,
                text_field_chrome_system,
                text_field_clear_system,
                text_field_disabled_system,
                text_field_cursor_style_system,
            ),
        );
    }
}
