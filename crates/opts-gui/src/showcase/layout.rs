//! Layout helpers of the showcase: columns, rows and headings.

use bevy::prelude::*;

use crate::components::label::LabelBuilder;
use crate::theme::typography::TypeScale;

pub(super) fn column(commands: &mut Commands, gap: f32) -> Entity {
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(gap),
            align_items: AlignItems::FlexStart,
            ..default()
        })
        .id()
}

pub(super) fn row(commands: &mut Commands, gap: f32) -> Entity {
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(gap),
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id()
}

pub(super) fn heading(commands: &mut Commands, text: &str) -> Entity {
    LabelBuilder::new(text)
        .with_style(TypeScale::TitleMedium)
        .spawn(commands)
}
