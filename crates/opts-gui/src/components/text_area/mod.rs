//! Multiline text input.
//!
//! [`TextArea`] owns a [`TextBuffer`] and renders it as wrapped text with a blinking cursor and a
//! highlighted selection. It consumes layout-aware keyboard events (so umlauts, dead keys and
//! emoji input work), supports the usual editing keys and `Ctrl+A/C/X/V`, and scrolls vertically
//! to keep the cursor visible. Tab is *not* consumed; it moves focus.

mod buffer;
mod clipboard;
mod input;
mod view;

pub use buffer::TextBuffer;
pub use clipboard::TextClipboard;
pub use input::{text_area_focus_system, text_area_input_system, TextAreaChanged};
pub use view::{text_area_render_system, TextAreaSpan, TextAreaText};

use crate::components::Button;
use bevy::prelude::*;

use crate::{
    interaction::{focus::Focusable, state::Focused},
    theme::{color::ColorRole, shape::RadiusLevel},
    tokens::components::{BackgroundRole, BorderRole, RadiusToken},
};

/// A focusable multiline text input. Read the content with [`TextArea::value`].
#[derive(Component, Debug, Clone, Default)]
pub struct TextArea {
    /// The edited text and cursor.
    pub buffer: TextBuffer,
    /// Whether the area currently receives keyboard input.
    pub focused: bool,
    /// Text shown (dimmed) while the buffer is empty.
    pub placeholder: String,
    /// Vertical scroll offset in pixels, maintained by the render system.
    pub scroll_px: f32,
    /// Seconds since the last edit or cursor move (drives the blink).
    pub blink_clock: f32,
}

impl TextArea {
    /// The current text.
    #[must_use]
    pub fn value(&self) -> &str {
        self.buffer.text()
    }
}

/// Builder for a [`TextArea`].
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn(mut commands: Commands) {
///     TextAreaBuilder::new()
///         .with_placeholder("Describe the issue")
///         .autofocus(true)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TextAreaBuilder {
    text: String,
    placeholder: String,
    autofocus: bool,
    tab_index: usize,
}

impl TextAreaBuilder {
    /// Creates an empty text area builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the initial text (cursor at the end).
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the placeholder shown while empty.
    #[must_use]
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Focuses the area on spawn so the user can type immediately.
    #[must_use]
    pub fn autofocus(mut self, on: bool) -> Self {
        self.autofocus = on;
        self
    }

    /// Sets the tab-order index.
    #[must_use]
    pub fn with_tab_index(mut self, index: usize) -> Self {
        self.tab_index = index;
        self
    }

    fn components(self) -> (impl Bundle, impl Bundle) {
        let style = Node {
            flex_grow: 1.0,
            width: Val::Percent(100.0),
            min_height: Val::Px(80.0),
            padding: UiRect::all(Val::Px(12.0)),
            overflow: Overflow::clip(),
            ..default()
        };
        (
            (
                (Button, style),
                TextArea {
                    buffer: TextBuffer::new(self.text),
                    focused: self.autofocus,
                    placeholder: self.placeholder,
                    scroll_px: 0.0,
                    blink_clock: 0.0,
                },
            ),
            (
                BackgroundRole(ColorRole::SurfaceVariant),
                BorderRole(ColorRole::Outline),
                RadiusToken(RadiusLevel::Small),
                Focusable {
                    tab_index: self.tab_index,
                },
            ),
        )
    }

    /// Spawns the text node: the text before the cursor, with two child spans for the
    /// cursor/selection and the text after it (see `view.rs`).
    fn spawn_inner_text(p: &mut ChildSpawnerCommands) {
        p.spawn((
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextLayout::new(Justify::Left, LineBreak::WordBoundary),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                ..default()
            },
            TextAreaText,
        ))
        .with_children(|t| {
            for kind in [TextAreaSpan::Middle, TextAreaSpan::After] {
                t.spawn((
                    TextSpan::new(""),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    kind,
                ));
            }
        });
    }

    /// Spawns the text area using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let autofocus = self.autofocus;
        let (a, b) = self.components();
        let root = commands.spawn((a, b)).id();
        if autofocus {
            commands.entity(root).insert(Focused);
        }
        commands.entity(root).with_children(|p| {
            Self::spawn_inner_text(p);
        });
        root
    }

    /// Spawns the text area as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let autofocus = self.autofocus;
        let (a, b) = self.components();
        let mut cmd = parent.spawn((a, b));
        if autofocus {
            cmd.insert(Focused);
        }
        cmd.with_children(|p| {
            Self::spawn_inner_text(p);
        });
        cmd.id()
    }
}

/// Plugin registering the [`TextArea`] systems.
#[derive(Default)]
pub struct TextAreaPlugin;

impl Plugin for TextAreaPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TextAreaChanged>()
            .init_non_send::<TextClipboard>()
            .add_systems(
                Update,
                (
                    text_area_focus_system,
                    text_area_input_system,
                    text_area_render_system,
                )
                    .chain(),
            );
    }
}
