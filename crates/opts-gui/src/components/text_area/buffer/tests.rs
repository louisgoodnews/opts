//! Unit tests for `super` (`buffer.rs`).

use super::*;

#[test]
fn insert_and_backspace_handle_multibyte() {
    let mut b = TextBuffer::default();
    b.insert_str("äö😀");
    assert_eq!(b.char_count(), 3);
    b.backspace();
    assert_eq!(b.text(), "äö");
    b.move_left(false);
    b.delete();
    assert_eq!(b.text(), "ä");
}

#[test]
fn newline_and_vertical_motion_keep_the_column() {
    let mut b = TextBuffer::new("abcdef\nxy\nlonger line");
    b.move_doc(false, false);
    for _ in 0..5 {
        b.move_right(false);
    }
    b.move_down(false);
    assert_eq!(b.cursor_line_col(), (1, 2)); // clamped to the short line
    b.move_down(false);
    assert_eq!(b.cursor_line_col(), (2, 5)); // remembered column restored
    b.move_up(false);
    b.move_up(false);
    b.move_up(false);
    assert_eq!(b.cursor(), 0);
}

#[test]
fn home_end_and_document_bounds() {
    let mut b = TextBuffer::new("one\ntwo");
    b.move_up(false);
    b.end(false);
    assert_eq!(b.cursor_line_col(), (0, 3));
    b.move_down(false);
    b.home(false);
    assert_eq!(b.cursor_line_col(), (1, 0));
    b.move_doc(true, false);
    assert_eq!(b.cursor(), 7);
}

#[test]
fn selection_replace_and_collapse() {
    let mut b = TextBuffer::new("hello world");
    b.move_doc(false, false);
    for _ in 0..5 {
        b.move_right(true);
    }
    assert_eq!(b.selected_text().as_deref(), Some("hello"));
    b.insert_str("Hi");
    assert_eq!(b.text(), "Hi world");
    b.select_all();
    b.move_left(false);
    assert_eq!(b.cursor(), 0);
    assert!(b.selection().is_none());
}

#[test]
fn carriage_returns_are_normalised() {
    let mut b = TextBuffer::default();
    b.insert_str("a\r\nb\rc");
    assert_eq!(b.text(), "a\nb\nc");
}

#[test]
fn render_split_marks_the_selection() {
    let mut b = TextBuffer::new("abcd");
    b.move_left(false);
    b.move_left(true);
    assert_eq!(b.split_for_render(), ("ab".into(), "c".into(), "d".into()));
}
