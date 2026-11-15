//! Cursor motion of the text buffer: by character, by line and to the ends of the text.

use super::TextBuffer;

impl TextBuffer {
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
}
