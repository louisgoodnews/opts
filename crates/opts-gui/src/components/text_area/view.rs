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

/// Rebuilds the visible text, blinks the cursor on real time and keeps the cursor in view.
pub fn text_area_render_system(
    real: Res<Time<Real>>,
    theme: Res<MaterialTheme>,
    mut areas: Query<(&mut TextArea, &Node, &Children)>,
    mut texts: Query<(&mut Text, &mut Style), With<TextAreaText>>,
) {
    let dt = real.delta_seconds();
    let color = |role| theme.scheme.get(role);
    for (mut area, node, children) in &mut areas {
        area.blink_clock += dt;
        let cursor_on = area.focused && (area.blink_clock % BLINK_PERIOD) < BLINK_PERIOD * 0.5;

        let width = (node.size().x - 2.0 * PAD).max(FONT_SIZE);
        let cpl = ((width / (FONT_SIZE * GLYPH_WIDTH)) as usize).max(1);
        let visible_h = (node.size().y - 2.0 * PAD).max(LINE_HEIGHT);
        let cur_y = cursor_visual_line(&area, cpl) as f32 * LINE_HEIGHT;
        let max_scroll = (total_visual_lines(&area, cpl) as f32 * LINE_HEIGHT - visible_h).max(0.0);
        let mut scroll = area.scroll_px.min(max_scroll);
        if cur_y < scroll {
            scroll = cur_y;
        } else if cur_y + LINE_HEIGHT > scroll + visible_h {
            scroll = cur_y + LINE_HEIGHT - visible_h;
        }
        if (scroll - area.scroll_px).abs() > f32::EPSILON {
            area.scroll_px = scroll;
        }

        let base = TextStyle {
            font_size: FONT_SIZE,
            color: color(ColorRole::OnSurface),
            ..default()
        };
        let sections = if area.buffer.text().is_empty() && !area.focused {
            vec![TextSection::new(
                area.placeholder.clone(),
                TextStyle {
                    color: color(ColorRole::OnSurfaceVariant),
                    ..base
                },
            )]
        } else {
            let (before, selected, after) = area.buffer.split_for_render();
            let cursor_color = if cursor_on {
                color(ColorRole::Primary)
            } else {
                Color::NONE
            };
            let mut s = vec![TextSection::new(before, base.clone())];
            if selected.is_empty() {
                s.push(TextSection::new(
                    "|",
                    TextStyle {
                        color: cursor_color,
                        ..base.clone()
                    },
                ));
            } else {
                s.push(TextSection::new(
                    selected,
                    TextStyle {
                        color: color(ColorRole::Primary),
                        ..base.clone()
                    },
                ));
            }
            s.push(TextSection::new(after, base));
            s
        };

        for &child in children {
            if let Ok((mut text, mut style)) = texts.get_mut(child) {
                if text.sections.len() != sections.len()
                    || text
                        .sections
                        .iter()
                        .zip(&sections)
                        .any(|(a, b)| a.value != b.value || a.style.color != b.style.color)
                {
                    text.sections = sections.clone();
                }
                let top = Val::Px(PAD - area.scroll_px);
                if style.top != top {
                    style.top = top;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::text_area::TextBuffer;

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
