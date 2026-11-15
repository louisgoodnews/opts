//! Editing of the text buffer: insertion, deletion and the selection.

use super::TextBuffer;

impl TextBuffer {
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
}
