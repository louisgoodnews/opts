//! Material Design 3 Treeview hierarchical tree navigation widget.

mod builder;
mod spawn;
mod systems;

pub use builder::TreeviewBuilder;
pub use systems::{treeview_arrow_system, treeview_click_system, treeview_sync_system};

use bevy::prelude::*;

use crate::interaction::events::{TreeNodeSelected, TreeNodeToggled};

/// Recursive tree node specification used for building tree views.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
    pub is_expanded: bool,
}

impl TreeNode {
    /// Creates a leaf tree node with the specified ID and label.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            is_expanded: false,
        }
    }

    /// Adds a child node to this node and marks it as expandable.
    pub fn with_child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    /// Sets whether this node is initially expanded.
    pub fn expanded(mut self, is_expanded: bool) -> Self {
        self.is_expanded = is_expanded;
        self
    }
}

/// Marker component for an M3 Treeview root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Treeview {
    pub selected_node_id: Option<String>,
}

/// Marker component for an individual node row in the treeview.
#[derive(Component, Debug, Clone)]
pub struct M3TreeNode {
    pub tree_entity: Entity,
    pub node_id: String,
    pub parent_id: Option<String>,
    pub has_children: bool,
    pub is_expanded: bool,
}

/// Marker component for the chevron toggle icon inside a node row.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeNodeChevron(pub Entity);

/// Plugin registering Treeview widget systems and events.
#[derive(Default)]
pub struct TreeviewPlugin;

impl Plugin for TreeviewPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<Messages<TreeNodeSelected>>()
        {
            app.add_message::<TreeNodeSelected>();
        }
        if !app.world().contains_resource::<Messages<TreeNodeToggled>>() {
            app.add_message::<TreeNodeToggled>();
        }
        app.add_systems(
            Update,
            (
                treeview_click_system,
                treeview_arrow_system,
                treeview_sync_system,
            ),
        );
    }
}

#[cfg(test)]
mod tests;
