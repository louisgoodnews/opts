//! Builder for the Material Design 3 treeview: the root entity and the public spawn entry points.

use bevy::prelude::*;

use super::{M3Treeview, TreeNode};

/// Builder for constructing Material Design 3 treeviews.
///
/// # Example
/// ```rust
/// use opts_gui::prelude::*;
/// use opts_gui::components::treeview::TreeNode;
/// use bevy::prelude::*;
///
/// fn spawn_tree(mut commands: Commands) {
///     let root_node = TreeNode::new("network", "Berlin Network")
///         .with_child(TreeNode::new("m29", "Line M29"))
///         .with_child(TreeNode::new("100", "Line 100"));
///
///     TreeviewBuilder::new()
///         .with_node(root_node)
///         .spawn(&mut commands);
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct TreeviewBuilder {
    pub root_nodes: Vec<TreeNode>,
}

impl TreeviewBuilder {
    /// Creates a new empty treeview builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a root tree node to the view.
    pub fn with_node(mut self, node: TreeNode) -> Self {
        self.root_nodes.push(node);
        self
    }

    /// Spawns the treeview into the Bevy ECS world using top-level [`Commands`].
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let root = commands
            .spawn((
                (Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                }),
                M3Treeview::default(),
            ))
            .id();

        commands.entity(root).with_children(|parent| {
            for node in &self.root_nodes {
                Self::spawn_node_hierarchy(parent, root, None, node, 0);
            }
        });

        root
    }

    /// Spawns the treeview as a child inside a parent [`ChildSpawnerCommands`].
    pub fn spawn_child(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let mut root_cmd = parent.spawn((
            (Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            }),
            M3Treeview::default(),
        ));

        let root = root_cmd.id();

        root_cmd.with_children(|child_parent| {
            for node in &self.root_nodes {
                Self::spawn_node_hierarchy(child_parent, root, None, node, 0);
            }
        });

        root
    }
}
