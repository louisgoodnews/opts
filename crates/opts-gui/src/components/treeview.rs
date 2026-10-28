//! Material Design 3 Treeview hierarchical tree navigation widget.

use crate::components::Button;
use bevy::prelude::*;

use crate::{
    actions::{Direction, UiAction},
    interaction::{
        activation::Activated,
        events::{TreeNodeSelected, TreeNodeToggled},
        focus::FocusEvent,
        state::{Disabled, Focused},
    },
    theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale},
    tokens::components::{ForegroundRole, RadiusToken, TypeStyleToken},
};

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

    fn spawn_node_hierarchy(
        parent: &mut ChildSpawnerCommands,
        tree_entity: Entity,
        parent_id: Option<String>,
        node: &TreeNode,
        depth: usize,
    ) {
        let has_children = !node.children.is_empty();
        let chevron_symbol = if !has_children {
            " "
        } else if node.is_expanded {
            "▼"
        } else {
            "▶"
        };

        let node_id = node.id.clone();

        // Node row button
        let mut row_cmd = parent.spawn((
            (
                Button,
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    min_height: Val::Px(32.0),
                    padding: UiRect {
                        left: Val::Px((depth as f32) * 20.0 + 8.0),
                        right: Val::Px(8.0),
                        top: Val::Px(4.0),
                        bottom: Val::Px(4.0),
                    },
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ),
            M3TreeNode {
                tree_entity,
                node_id: node_id.clone(),
                parent_id,
                has_children,
                is_expanded: node.is_expanded,
            },
            RadiusToken(RadiusLevel::ExtraSmall),
        ));

        let row_entity = row_cmd.id();

        row_cmd.with_children(|row_parent| {
            // Chevron indicator
            row_parent.spawn((
                (
                    Text::new(chevron_symbol),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor::default(),
                ),
                TreeNodeChevron(row_entity),
                crate::ui_tree::A11yHidden,
                ForegroundRole(ColorRole::OnSurfaceVariant),
                TypeStyleToken(TypeScale::Label),
            ));

            // Node label text
            row_parent.spawn((
                (
                    Text::new(node.label.clone()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor::default(),
                ),
                ForegroundRole(ColorRole::OnSurface),
                TypeStyleToken(TypeScale::BodyMedium),
            ));
        });

        // Recursively spawn children
        for child in &node.children {
            Self::spawn_node_hierarchy(
                parent,
                tree_entity,
                Some(node_id.clone()),
                child,
                depth + 1,
            );
        }
    }
}

/// System handling clicks on tree node rows to expand/collapse and select nodes.
pub fn treeview_click_system(
    mut activated: MessageReader<Activated>,
    mut click_query: Query<&mut M3TreeNode, Without<Disabled>>,
    mut tree_query: Query<&mut M3Treeview>,
    mut select_events: MessageWriter<TreeNodeSelected>,
    mut toggle_events: MessageWriter<TreeNodeToggled>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(mut node) = click_query.get_mut(*entity) {
            // Selection event
            if let Ok(mut tree) = tree_query.get_mut(node.tree_entity) {
                tree.selected_node_id = Some(node.node_id.clone());
                select_events.write(TreeNodeSelected {
                    tree_entity: node.tree_entity,
                    node_id: node.node_id.clone(),
                });
            }

            // Toggle expansion if expandable
            if node.has_children {
                node.is_expanded = !node.is_expanded;
                toggle_events.write(TreeNodeToggled {
                    tree_entity: node.tree_entity,
                    node_id: node.node_id.clone(),
                    is_expanded: node.is_expanded,
                });
            }
        }
    }
}

/// Left/Right on a focused tree row: Right expands (a collapsed parent), Left collapses (an
/// expanded parent) or moves focus to the parent row.
pub fn treeview_arrow_system(
    mut actions: MessageReader<UiAction>,
    mut nodes: Query<(Entity, &mut M3TreeNode, Has<Focused>)>,
    mut toggle_events: MessageWriter<TreeNodeToggled>,
    mut focus: MessageWriter<FocusEvent>,
) {
    for action in actions.read() {
        let dir = match action {
            UiAction::Navigate(d @ (Direction::Left | Direction::Right)) => *d,
            _ => continue,
        };
        let Some((_, tree, id, parent_id, has_children, expanded)) =
            nodes.iter().find(|(_, _, f)| *f).map(|(e, n, _)| {
                (
                    e,
                    n.tree_entity,
                    n.node_id.clone(),
                    n.parent_id.clone(),
                    n.has_children,
                    n.is_expanded,
                )
            })
        else {
            continue;
        };
        let wants_toggle = match dir {
            Direction::Right => has_children && !expanded,
            _ => has_children && expanded,
        };
        if wants_toggle {
            for (_, mut node, _) in &mut nodes {
                if node.tree_entity == tree && node.node_id == id {
                    node.is_expanded = !expanded;
                    toggle_events.write(TreeNodeToggled {
                        tree_entity: tree,
                        node_id: id.clone(),
                        is_expanded: node.is_expanded,
                    });
                }
            }
        } else if dir == Direction::Left {
            if let Some(parent) = nodes
                .iter()
                .find(|(_, n, _)| n.tree_entity == tree && Some(&n.node_id) == parent_id.as_ref())
            {
                focus.write(FocusEvent::Set(parent.0));
            }
        }
    }
}

/// System synchronizing chevron symbols and child visibility when a node expands/collapses.
pub fn treeview_sync_system(
    changed_nodes: Query<(Entity, &M3TreeNode), Changed<M3TreeNode>>,
    mut chevron_query: Query<(&TreeNodeChevron, &mut Text)>,
    mut all_nodes: Query<(&M3TreeNode, &mut Node)>,
) {
    for (entity, changed_node) in &changed_nodes {
        // Update chevron glyph
        let glyph = if !changed_node.has_children {
            " "
        } else if changed_node.is_expanded {
            "▼"
        } else {
            "▶"
        };

        for (chevron, mut text) in &mut chevron_query {
            if chevron.0 == entity {
                text.0 = glyph.to_string();
            }
        }

        // Show/hide direct children
        let parent_id_str = changed_node.node_id.clone();
        let show_children = changed_node.is_expanded;

        for (node, mut style) in &mut all_nodes {
            if node.parent_id.as_ref() == Some(&parent_id_str) {
                style.display = if show_children {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}

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
