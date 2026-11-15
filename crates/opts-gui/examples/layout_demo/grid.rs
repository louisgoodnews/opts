//! The CSS-grid cell of the layout demo: a two-column grid of four coloured cells.

use bevy::prelude::*;
use opts_gui::prelude::*;

pub(super) fn spawn_css_grid(root: &mut ChildSpawnerCommands) {
    root.spawn((
        GridBuilder::new()
            .with_columns(2)
            .with_gap(SpacingScale::Small)
            .build(),
        BackgroundRole(ColorRole::SurfaceContainerLow),
        RadiusToken(RadiusLevel::Medium),
        BorderRole(ColorRole::OutlineVariant),
    ))
    .with_children(|grid| {
        for i in 1..=4 {
            grid.spawn((
                (Node {
                    width: Val::Px(70.0),
                    height: Val::Px(70.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                }),
                BackgroundRole(ColorRole::TertiaryContainer),
                RadiusToken(RadiusLevel::Small),
            ))
            .with_children(|cell| {
                cell.spawn((
                    Text::new(format!("Cell {i}")),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
            });
        }
    });
}
