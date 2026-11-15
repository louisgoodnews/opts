//! Builder for the Material Design 3 notebook / tab container.

use bevy::prelude::*;

use super::spawn::populate_notebook_children;
use super::M3Notebook;

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

    pub(super) fn root_bundle(&self) -> impl Bundle {
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
