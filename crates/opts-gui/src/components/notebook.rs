//! Material Design 3 Notebook / Tabbed container widget.

use crate::components::Button;
use bevy::prelude::*;

use crate::{
    interaction::{activation::Activated, events::NotebookTabSelected, state::Disabled},
    theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale},
    tokens::components::{BackgroundRole, BorderRole, ForegroundRole, RadiusToken, TypeStyleToken},
};

/// Marker component for an M3 Notebook tabbed container root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Notebook {
    pub active_tab: usize,
    pub tab_count: usize,
}

/// Marker component for an individual tab header button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookTabHeader {
    pub notebook: Entity,
    pub index: usize,
}

/// Marker component for the active indicator underline of a tab header.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookTabIndicator {
    pub notebook: Entity,
    pub index: usize,
}

/// Marker component for a content panel associated with a tab.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookPanel {
    pub notebook: Entity,
    pub index: usize,
}

/// Builder for constructing Material Design 3 notebook / tab containers.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use bevy::prelude::*;
///
/// fn spawn_notebook(mut commands: Commands) {
///     NotebookBuilder::new()
///         .with_tabs(vec!["Timetable".into(), "Fleet".into(), "Staff".into()])
///         .with_active(0)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct NotebookBuilder {
    pub tabs: Vec<String>,
    pub active_tab: usize,
}

impl NotebookBuilder {
    /// Creates a new empty notebook builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a tab title to the notebook.
    pub fn with_tab(mut self, title: impl Into<String>) -> Self {
        self.tabs.push(title.into());
        self
    }

    /// Sets the list of tab titles.
    pub fn with_tabs(mut self, tabs: Vec<String>) -> Self {
        self.tabs = tabs;
        self
    }

    /// Sets the initially active tab index.
    pub fn with_active(mut self, active_tab: usize) -> Self {
        if active_tab < self.tabs.len() {
            self.active_tab = active_tab;
        }
        self
    }

    fn root_bundle(&self) -> impl Bundle {
        (
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            }),
            M3Notebook {
                active_tab: self.active_tab,
                tab_count: self.tabs.len(),
            },
        )
    }

    /// Spawns the notebook into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        self.spawn_with_panels(commands).0
    }

    /// Spawns the notebook into the Bevy ECS world and returns `(root_entity, panel_entities)`.
    pub fn spawn_with_panels(self, commands: &mut Commands) -> (Entity, Vec<Entity>) {
        let tabs = self.tabs.clone();
        let active = self.active_tab;
        let mut panels = Vec::new();

        let root = commands.spawn(self.root_bundle()).id();

        commands.entity(root).with_children(|parent| {
            panels = populate_notebook_children(parent, root, &tabs, active);
        });

        (root, panels)
    }

    /// Spawns the notebook as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let tabs = self.tabs.clone();
        let active = self.active_tab;

        let mut root_cmd = parent.spawn(self.root_bundle());
        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            let _ = populate_notebook_children(child_parent, root, &tabs, active);
        });

        root
    }
}

fn populate_notebook_children(
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
                let is_active = i == active;
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
                                TextColor(Color::WHITE),
                            ),
                            ForegroundRole(if is_active {
                                ColorRole::Primary
                            } else {
                                ColorRole::OnSurfaceVariant
                            }),
                            TypeStyleToken(TypeScale::TitleSmall),
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
        });

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

/// System handling clicks on tab headers to switch the active tab.
pub fn notebook_tab_click_system(
    mut activated: MessageReader<Activated>,
    header_query: Query<&NotebookTabHeader, Without<Disabled>>,
    mut notebook_query: Query<&mut M3Notebook>,
    mut events: MessageWriter<NotebookTabSelected>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(header) = header_query.get(*entity) {
            if let Ok(mut nb) = notebook_query.get_mut(header.notebook) {
                if nb.active_tab != header.index {
                    nb.active_tab = header.index;
                    events.write(NotebookTabSelected {
                        entity: header.notebook,
                        tab_index: header.index,
                    });
                }
            }
        }
    }
}

/// System synchronizing tab indicator visibility and panel visibility with the active tab.
pub fn notebook_sync_system(
    nb_query: Query<(Entity, &M3Notebook), Changed<M3Notebook>>,
    mut indicator_query: Query<(&NotebookTabIndicator, &mut Node), Without<NotebookPanel>>,
    mut panel_query: Query<(&NotebookPanel, &mut Node), Without<NotebookTabIndicator>>,
) {
    for (entity, nb) in &nb_query {
        for (indicator, mut style) in &mut indicator_query {
            if indicator.notebook == entity {
                style.display = if indicator.index == nb.active_tab {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }

        for (panel, mut style) in &mut panel_query {
            if panel.notebook == entity {
                style.display = if panel.index == nb.active_tab {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

/// Plugin registering Notebook widget systems and events.
#[derive(Default)]
pub struct NotebookPlugin;

impl Plugin for NotebookPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<Messages<NotebookTabSelected>>()
        {
            app.add_message::<NotebookTabSelected>();
        }
        app.add_systems(Update, (notebook_tab_click_system, notebook_sync_system));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notebook_builder_spawn() {
        let mut app = App::new();
        let entity = {
            let mut commands = app.world_mut().commands();
            NotebookBuilder::new()
                .with_tabs(vec!["Tab 0".into(), "Tab 1".into()])
                .with_active(1)
                .spawn(&mut commands)
        };

        app.update();

        let nb = app.world().get::<M3Notebook>(entity);
        assert_eq!(nb.map(|n| n.active_tab), Some(1));
        assert_eq!(nb.map(|n| n.tab_count), Some(2));
    }
}
