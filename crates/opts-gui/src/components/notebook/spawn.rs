//! Spawning of the tab header bar and the content panels of a notebook.

use bevy::prelude::*;

use super::{NotebookPanel, NotebookTabHeader, NotebookTabIndicator};
use crate::components::Button;
use crate::{
    theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

/// One tab header: its title and the underline that shows while it is the active one.
fn tab_header(
    bar_parent: &mut ChildSpawnerCommands,
    root: Entity,
    (i, title): (usize, &String),
    is_active: bool,
) {
    bar_parent
        .spawn((
            (
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(16.0), Val::Px(12.0)),
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    position_type: PositionType::Relative,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            NotebookTabHeader {
                notebook: root,
                index: i,
            },
        ))
        .with_children(|tab_parent| {
            tab_parent.spawn((
                (
                    Text::new(title.clone()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor::default(),
                ),
                ForegroundRole(if is_active {
                    ColorRole::Primary
                } else {
                    ColorRole::OnSurfaceVariant
                }),
                TypeStyleToken(TypeScale::BodyMedium),
            ));

            // Active indicator underline
            tab_parent.spawn((
                (Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(3.0),
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    display: if is_active {
                        Display::Flex
                    } else {
                        Display::None
                    },
                    ..default()
                }),
                NotebookTabIndicator {
                    notebook: root,
                    index: i,
                },
                BackgroundRole(ColorRole::Primary),
                RadiusToken(RadiusLevel::Full),
            ));
        });
}

/// The container of the content panels, one per tab; returns the panels.
fn spawn_panels(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    (tab_count, active): (usize, usize),
) -> Vec<Entity> {
    let mut panels = Vec::with_capacity(tab_count);

    // Content panels container
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|content_parent| {
            for i in 0..tab_count {
                let is_active = i == active;
                let panel_id = content_parent
                    .spawn((
                        (Node {
                            display: if is_active {
                                Display::Flex
                            } else {
                                Display::None
                            },
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            width: Val::Percent(100.0),
                            padding: UiRect::all(Val::Px(16.0)),
                            ..default()
                        }),
                        NotebookPanel {
                            notebook: root,
                            index: i,
                        },
                    ))
                    .id();
                panels.push(panel_id);
            }
        });
    panels
}

pub(super) fn populate_notebook_children(
    parent: &mut ChildSpawnerCommands,
    root: Entity,
    tabs: &[String],
    active: usize,
) -> Vec<Entity> {
    let tab_count = tabs.len();

    // Tab header bar
    parent
        .spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                min_height: Val::Px(48.0),
                border: UiRect {
                    bottom: Val::Px(1.0),
                    ..default()
                },
                ..default()
            }),
            BackgroundRole(ColorRole::Surface),
            BorderRole(ColorRole::OutlineVariant),
        ))
        .with_children(|bar_parent| {
            for (i, title) in tabs.iter().enumerate() {
                tab_header(bar_parent, root, (i, title), i == active);
            }
        });

    spawn_panels(parent, root, (tab_count, active))
}
