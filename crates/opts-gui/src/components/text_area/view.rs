//! Rendering of [`TextArea`]: text sections, cursor blink, selection colour and scrolling.

use bevy::prelude::*;

use super::TextArea;
use crate::theme::{color::ColorRole, MaterialTheme};

const FONT_SIZE: f32 = 16.0;
const LINE_HEIGHT: f32 = FONT_SIZE * 1.25;
const PAD: f32 = 12.0;
/// Average glyph width relative to the font size, used to estimate wrapped line counts.
const GLYPH_WIDTH: f32 = 0.52;
const BLINK_PERIOD: f32 = 1.0;

/// Marker for the text child that displays a [`TextArea`]'s content.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TextAreaText;

/// The child spans of the text node: cursor or selection, and the text after it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaSpan {
    /// The cursor bar or the selected text.
    Middle,
    /// The text after the cursor/selection.
    After,
}

/// Estimated visual (wrapped) line of the cursor, counted from the top.
fn cursor_visual_line(area: &TextArea, chars_per_line: usize) -> usize {
    let (line, col) = area.buffer.cursor_line_col();
    let mut visual = 0;
    for l in area.buffer.text().split('\n').take(line) {
        visual += l.chars().count().div_ceil(chars_per_line).max(1);
    }
    visual + col / chars_per_line
}

/// Estimated number of visual (wrapped) lines of the whole text.
fn total_visual_lines(area: &TextArea, chars_per_line: usize) -> usize {
    area.buffer
        .text()
        .split('\n')
        .map(|l| l.chars().count().div_ceil(chars_per_line).max(1))
        .sum()
}

fn update_scroll(area: &mut TextArea, computed: &ComputedNode) {
    let size = computed.size() * computed.inverse_scale_factor();
    let width = (size.x - 2.0 * PAD).max(FONT_SIZE);
    let cpl = ((width / (FONT_SIZE * GLYPH_WIDTH)) as usize).max(1);
    let visible_h = (size.y - 2.0 * PAD).max(LINE_HEIGHT);
    let cur_y = cursor_visual_line(area, cpl) as f32 * LINE_HEIGHT;
    let max_scroll = (total_visual_lines(area, cpl) as f32 * LINE_HEIGHT - visible_h).max(0.0);
    let mut scroll = area.scroll_px.min(max_scroll);
    if cur_y < scroll {
        scroll = cur_y;
    } else if cur_y + LINE_HEIGHT > scroll + visible_h {
        scroll = cur_y + LINE_HEIGHT - visible_h;
    }
    if (scroll - area.scroll_px).abs() > f32::EPSILON {
        area.scroll_px = scroll;
    }
}

struct RenderSegments {
    before: String,
    middle: String,
    after: String,
    middle_color: Color,
    base_color: Color,
}

fn split_render_segments(
    area: &TextArea,
    cursor_on: bool,
    theme: &MaterialTheme,
) -> RenderSegments {
    let color = |role| theme.scheme.get(role);
    if area.buffer.text().is_empty() && !area.focused {
        RenderSegments {
            before: area.placeholder.clone(),
            middle: String::new(),
            after: String::new(),
            middle_color: Color::NONE,
            base_color: color(ColorRole::OnSurfaceVariant),
        }
    } else {
        let (before, selected, after) = area.buffer.split_for_render();
        if selected.is_empty() {
            let cursor = if cursor_on {
                color(ColorRole::Primary)
            } else {
                Color::NONE
            };
            RenderSegments {
                before,
                middle: "|".to_owned(),
                after,
                middle_color: cursor,
                base_color: color(ColorRole::OnSurface),
            }
        } else {
            RenderSegments {
                before,
                middle: selected,
                after,
                middle_color: color(ColorRole::Primary),
                base_color: color(ColorRole::OnSurface),
            }
        }
    }
}

/// Rebuilds the visible text, blinks the cursor on real time and keeps the cursor in view.
pub fn text_area_render_system(
    real: Res<Time<Real>>,
    theme: Res<MaterialTheme>,
    mut areas: Query<(&mut TextArea, &ComputedNode, &Children)>,
    mut texts: Query<(&mut Text, &mut TextColor, &mut Node, &Children), With<TextAreaText>>,
    mut spans: Query<(&mut TextSpan, &mut TextColor, &TextAreaSpan), Without<TextAreaText>>,
) {
    let dt = real.delta_secs();
    for (mut area, computed, children) in &mut areas {
        area.blink_clock += dt;
        let cursor_on = area.focused && (area.blink_clock % BLINK_PERIOD) < BLINK_PERIOD * 0.5;

        update_scroll(&mut area, computed);
        let segs = split_render_segments(&area, cursor_on, &theme);

        for &child in children {
            let Ok((mut text, mut text_color, mut node, span_children)) = texts.get_mut(child)
            else {
                continue;
            };
            if text.0 != segs.before {
                text.0 = segs.before.clone();
            }
            if text_color.0 != segs.base_color {
                text_color.0 = segs.base_color;
            }
            for &span in span_children {
                let Ok((mut span_text, mut span_color, kind)) = spans.get_mut(span) else {
                    continue;
                };
                let (value, c) = match kind {
                    TextAreaSpan::Middle => (&segs.middle, segs.middle_color),
                    TextAreaSpan::After => (&segs.after, segs.base_color),
                };
                if span_text.0 != *value {
                    span_text.0 = value.clone();
                }
                if span_color.0 != c {
                    span_color.0 = c;
                }
            }
            let top = Val::Px(PAD - area.scroll_px);
            if node.top != top {
                node.top = top;
            }
        }
    }
}

#[cfg(test)]
mod tests {
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
}
