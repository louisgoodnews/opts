//! Unit tests of `view`.

use super::*;
use crate::components::text_area::TextBuffer;

/// WCAG 2.3.1: nothing flashes more than three times a second. The caret is the only
/// periodic visual change of the text widgets; it flashes once per period.
#[test]
fn the_caret_flashes_well_below_three_times_a_second() {
    const { assert!(1.0 / BLINK_PERIOD <= 2.0) };
}

#[test]
fn cursor_line_accounts_for_wrapping() {
    let area = TextArea {
        buffer: TextBuffer::new("12345678901\nab"),
        ..default()
    };
    // 10 chars per visual line: the first logical line wraps to 2 visual lines.
    assert_eq!(cursor_visual_line(&area, 10), 2);
}
