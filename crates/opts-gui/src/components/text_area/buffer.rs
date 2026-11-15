//! Pure text-editing model behind the multiline [`TextArea`](super::TextArea).
//!
//! Positions are **character** indices (Unicode scalar values), so umlauts and emoji move and
//! delete as one unit. Newlines are `\n`; the model never stores `\r`.

mod edit;
mod motion;

/// Editable multiline text with a cursor and an optional selection anchor.
///
/// # Example
/// ```rust
/// use opts_gui::components::text_area::TextBuffer;
///
/// let mut b = TextBuffer::default();
/// b.insert_str("Grüße");
/// b.insert_char('\n');
/// b.insert_str("zweite Zeile");
/// assert_eq!(b.line_count(), 2);
/// b.move_up(false);
/// assert_eq!(b.cursor_line_col().0, 0);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextBuffer {
    text: String,
    /// Cursor position as a character index in `0..=char_count`.
    cursor: usize,
    /// Selection anchor; the selection spans `anchor..cursor` (either order).
    anchor: Option<usize>,
    /// Column kept while moving vertically through shorter lines.
    wanted_col: Option<usize>,
}

impl TextBuffer {
    /// Creates a buffer holding `text` with the cursor at its end.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into().replace("\r\n", "\n").replace('\r', "\n");
        let cursor = text.chars().count();
        Self {
            text,
            cursor,
            anchor: None,
            wanted_col: None,
        }
    }

    /// The full text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Number of characters.
    #[must_use]
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    /// Cursor position in characters.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Number of logical lines (an empty buffer has one).
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.text.split('\n').count()
    }

    /// `(line, column)` of the cursor, both zero-based, in characters.
    #[must_use]
    pub fn cursor_line_col(&self) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        for (i, c) in self.text.chars().enumerate() {
            if i == self.cursor {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// The selected character range `(start, end)`, if any and non-empty.
    #[must_use]
    pub fn selection(&self) -> Option<(usize, usize)> {
        let a = self.anchor?;
        (a != self.cursor).then(|| (a.min(self.cursor), a.max(self.cursor)))
    }

    /// The selected text, if any.
    #[must_use]
    pub fn selected_text(&self) -> Option<String> {
        let (s, e) = self.selection()?;
        Some(self.text.chars().skip(s).take(e - s).collect())
    }

    fn byte_index(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map_or(self.text.len(), |(b, _)| b)
    }
    /// Splits the text into `(before selection/cursor, selected, after)` for rendering.
    #[must_use]
    pub fn split_for_render(&self) -> (String, String, String) {
        let (s, e) = self.selection().unwrap_or((self.cursor, self.cursor));
        let before: String = self.text.chars().take(s).collect();
        let mid: String = self.text.chars().skip(s).take(e - s).collect();
        let after: String = self.text.chars().skip(e).collect();
        (before, mid, after)
    }
}

#[cfg(test)]
mod tests;
