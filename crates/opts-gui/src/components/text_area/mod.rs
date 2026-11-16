//! Multiline text input.
//!
//! [`TextArea`] owns a [`TextBuffer`] and renders it as wrapped text with a blinking cursor and a
//! highlighted selection. It consumes layout-aware keyboard events (so umlauts, dead keys and
//! emoji input work), supports the usual editing keys and `Ctrl+A/C/X/V`, and scrolls vertically
//! to keep the cursor visible. Tab is *not* consumed; it moves focus.

mod buffer;
mod builder;
mod clipboard;
mod input;
mod view;

pub use buffer::TextBuffer;
pub use builder::TextAreaBuilder;
pub use clipboard::TextClipboard;
pub use input::{text_area_focus_system, text_area_input_system, TextAreaChanged};
pub use view::{text_area_render_system, TextAreaSpan, TextAreaText};

use bevy::prelude::*;

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
