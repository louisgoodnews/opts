//! Spawning of the node rows and the recursive node hierarchy of a treeview.

use bevy::prelude::*;

use super::{M3TreeNode, TreeNode, TreeNodeChevron, TreeviewBuilder};
use crate::components::Button;
use crate::theme::{color::ColorRole, shape::RadiusLevel, typography::TypeScale};
use crate::tokens::components::{ForegroundRole, RadiusToken, TypeStyleToken};

impl TreeviewBuilder {
    pub(super) fn spawn_tree_row(
        parent: &mut ChildSpawnerCommands,
        tree_entity: Entity,
        parent_id: Option<String>,
        node: &TreeNode,
        depth: usize,
        chevron_symbol: &'static str,
    ) -> Entity {
        let has_children = !node.children.is_empty();
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
                node_id: node.id.clone(),
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

        row_entity
    }

    pub(super) fn spawn_node_hierarchy(
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
        let _ = Self::spawn_tree_row(parent, tree_entity, parent_id, node, depth, chevron_symbol);

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
