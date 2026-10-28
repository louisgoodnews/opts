//! Pure text-editing model behind the multiline [`TextArea`](super::TextArea).
//!
//! Positions are **character** indices (Unicode scalar values), so umlauts and emoji move and
//! delete as one unit. Newlines are `\n`; the model never stores `\r`.

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

    /// Replaces the whole text (cursor to the end, selection cleared).
    pub fn set_text(&mut self, text: impl Into<String>) {
        *self = Self::new(text);
    }

    /// Deletes the selection and returns whether there was one.
    pub fn delete_selection(&mut self) -> bool {
        let Some((s, e)) = self.selection() else {
            self.anchor = None;
            return false;
        };
        let (bs, be) = (self.byte_index(s), self.byte_index(e));
        self.text.replace_range(bs..be, "");
        self.cursor = s;
        self.anchor = None;
        self.wanted_col = None;
        true
    }

    /// Inserts `s` at the cursor, replacing any selection. `\r\n` and `\r` become `\n`.
    pub fn insert_str(&mut self, s: &str) {
        let s = s.replace("\r\n", "\n").replace('\r', "\n");
        self.delete_selection();
        let b = self.byte_index(self.cursor);
        self.text.insert_str(b, &s);
        self.cursor += s.chars().count();
        self.wanted_col = None;
    }

    /// Inserts one character at the cursor.
    pub fn insert_char(&mut self, c: char) {
        let mut tmp = [0u8; 4];
        self.insert_str(c.encode_utf8(&mut tmp));
    }

    /// Deletes the selection, or the character before the cursor.
    pub fn backspace(&mut self) {
        if self.delete_selection() || self.cursor == 0 {
            return;
        }
        let (bs, be) = (
            self.byte_index(self.cursor - 1),
            self.byte_index(self.cursor),
        );
        self.text.replace_range(bs..be, "");
        self.cursor -= 1;
        self.wanted_col = None;
    }

    /// Deletes the selection, or the character after the cursor.
    pub fn delete(&mut self) {
        if self.delete_selection() || self.cursor >= self.char_count() {
            return;
        }
        let (bs, be) = (
            self.byte_index(self.cursor),
            self.byte_index(self.cursor + 1),
        );
        self.text.replace_range(bs..be, "");
        self.wanted_col = None;
    }

    /// Selects everything.
    pub fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.char_count();
    }

    fn begin_move(&mut self, extend: bool) {
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
    }

    /// Moves one character left. Without `extend`, a selection collapses to its start.
    pub fn move_left(&mut self, extend: bool) {
        if !extend {
            if let Some((s, _)) = self.selection() {
                self.cursor = s;
                self.anchor = None;
                self.wanted_col = None;
                return;
            }
        }
        self.begin_move(extend);
        self.cursor = self.cursor.saturating_sub(1);
        self.wanted_col = None;
    }

    /// Moves one character right. Without `extend`, a selection collapses to its end.
    pub fn move_right(&mut self, extend: bool) {
        if !extend {
            if let Some((_, e)) = self.selection() {
                self.cursor = e;
                self.anchor = None;
                self.wanted_col = None;
                return;
            }
        }
        self.begin_move(extend);
        self.cursor = (self.cursor + 1).min(self.char_count());
        self.wanted_col = None;
    }

    fn line_starts(&self) -> Vec<usize> {
        let mut starts = vec![0];
        for (i, c) in self.text.chars().enumerate() {
            if c == '\n' {
                starts.push(i + 1);
            }
        }
        starts
    }

    fn line_len(&self, line: usize) -> usize {
        self.text
            .split('\n')
            .nth(line)
            .map_or(0, |l| l.chars().count())
    }

    fn move_vertical(&mut self, delta: isize, extend: bool) {
        self.begin_move(extend);
        let (line, col) = self.cursor_line_col();
        let wanted = *self.wanted_col.get_or_insert(col);
        let target = line as isize + delta;
        if target < 0 {
            self.cursor = 0;
        } else if target as usize >= self.line_count() {
            self.cursor = self.char_count();
        } else {
            let t = target as usize;
            self.cursor = self.line_starts()[t] + wanted.min(self.line_len(t));
        }
    }

    /// Moves to the same column of the previous line (start of text from the first line).
    pub fn move_up(&mut self, extend: bool) {
        self.move_vertical(-1, extend);
    }

    /// Moves to the same column of the next line (end of text from the last line).
    pub fn move_down(&mut self, extend: bool) {
        self.move_vertical(1, extend);
    }

    /// Moves `lines` lines up (negative) or down (positive), as for PageUp/PageDown.
    pub fn move_lines(&mut self, lines: isize, extend: bool) {
        self.move_vertical(lines, extend);
    }

    /// Moves to the start of the current line.
    pub fn home(&mut self, extend: bool) {
        self.begin_move(extend);
        let (line, _) = self.cursor_line_col();
        self.cursor = self.line_starts()[line];
        self.wanted_col = None;
    }

    /// Moves to the end of the current line.
    pub fn end(&mut self, extend: bool) {
        self.begin_move(extend);
        let (line, _) = self.cursor_line_col();
        self.cursor = self.line_starts()[line] + self.line_len(line);
        self.wanted_col = None;
    }

    /// Moves to the start (`false`) or end (`true`) of the whole text.
    pub fn move_doc(&mut self, to_end: bool, extend: bool) {
        self.begin_move(extend);
        self.cursor = if to_end { self.char_count() } else { 0 };
        self.wanted_col = None;
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
