//! Systems of the treeview: click selection and expansion, arrow-key navigation, child visibility.

use bevy::prelude::*;

use super::{M3TreeNode, M3Treeview, TreeNodeChevron};
use crate::{
    actions::{Direction, UiAction},
    interaction::{
        activation::Activated,
        events::{TreeNodeSelected, TreeNodeToggled},
        focus::FocusEvent,
        state::{Disabled, Focused},
    },
};

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
