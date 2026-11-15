//! Title and subtitle of a master list row, cut to the row width.

use bevy::prelude::*;

use crate::patterns::list_model::ListEntry;
use crate::theme::color::ColorRole;
use crate::theme::typography::TypeScale;
use crate::tokens::components::{ForegroundRole, TypeStyleToken};

/// `text` cut to `max` characters with an ellipsis.
pub fn ellipsize(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub(super) fn spawn_texts(
    commands: &mut Commands,
    entry: &ListEntry,
    fg: ColorRole,
    max_chars: usize,
) -> Entity {
    let col = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                min_width: Val::Px(0.0),
                overflow: Overflow::clip(),
                ..default()
            },
            Name::new("RowTexts"),
        ))
        .id();
    let title = commands
        .spawn((
            Text::new(ellipsize(&entry.title, max_chars)),
            TextLayout {
                linebreak: LineBreak::NoWrap,
                ..default()
            },
            TextColor::default(),
            ForegroundRole(fg),
            TypeStyleToken(TypeScale::TitleMedium),
        ))
        .id();
    commands.entity(col).add_child(title);
    if !entry.subtitle.is_empty() {
        let sub = commands
            .spawn((
                Text::new(ellipsize(&entry.subtitle, max_chars + 6)),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                TextColor::default(),
                ForegroundRole(if fg == ColorRole::OnSurface {
                    ColorRole::OnSurfaceVariant
                } else {
                    fg
                }),
                TypeStyleToken(TypeScale::Label),
            ))
            .id();
        commands.entity(col).add_child(sub);
    }
    col
}
