//! Accessibility nodes of the elements with an id: role, label and state, refreshed ten times a second.

use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;

use crate::ui_tree::{label_of, role_of, UiRole};

use super::UiId;

fn accesskit_role(role: UiRole) -> Role {
    match role {
        UiRole::Button => Role::Button,
        UiRole::Checkbox => Role::CheckBox,
        UiRole::Radio => Role::RadioButton,
        UiRole::Switch => Role::Switch,
        UiRole::Textbox => Role::TextInput,
        UiRole::Combobox => Role::ComboBox,
        UiRole::Option => Role::ListBoxOption,
        UiRole::Tablist => Role::TabList,
        UiRole::Tab => Role::Tab,
        UiRole::Tabpanel => Role::TabPanel,
        UiRole::Tree => Role::Tree,
        UiRole::Treeitem => Role::TreeItem,
        UiRole::Listitem => Role::ListItem,
        UiRole::Dialog => Role::Dialog,
        UiRole::Toast => Role::Status,
        UiRole::Region => Role::Region,
    }
}

/// Writes role, label and state of every element with an id into its [`AccessibilityNode`].
pub fn sync_accessibility(world: &mut World, mut since_last: Local<Option<f32>>) {
    // Screen readers do not need 60 updates a second; 10 per second keeps 1,000 controls cheap.
    let waited = since_last.unwrap_or(f32::MAX) + world.resource::<Time<Real>>().delta_secs();
    let new_ids = world
        .query_filtered::<(), Added<UiId>>()
        .iter(world)
        .next()
        .is_some();
    if waited < 0.1 && !new_ids {
        *since_last = Some(waited);
        return;
    }
    *since_last = Some(0.0);
    let ids: Vec<Entity> = world
        .query_filtered::<Entity, With<UiId>>()
        .iter(world)
        .collect();
    for entity in ids {
        let Ok(e) = world.get_entity(entity) else {
            continue;
        };
        let Some(role) = role_of(&e) else { continue };
        let label = label_of(world, entity);
        let st = crate::ui_tree::state_snapshot(world, entity);
        let mut node = accesskit::Node::new(accesskit_role(role));
        if !label.is_empty() {
            node.set_label(label);
        }
        if let Some(c) = st.checked {
            node.set_toggled(match c {
                "true" => Toggled::True,
                "mixed" => Toggled::Mixed,
                _ => Toggled::False,
            });
        }
        if let Some(sel) = st.selected {
            node.set_selected(sel);
        }
        if let Some(open) = st.expanded {
            node.set_expanded(open);
        }
        if st.disabled.is_some() {
            node.set_disabled();
        }
        let mut entity_mut = world.entity_mut(entity);
        match entity_mut.get_mut::<AccessibilityNode>() {
            Some(mut existing) => {
                let differs = existing.role() != node.role()
                    || existing.label() != node.label()
                    || existing.toggled() != node.toggled()
                    || existing.is_selected() != node.is_selected()
                    || existing.is_expanded() != node.is_expanded()
                    || existing.is_disabled() != node.is_disabled();
                if differs {
                    existing.set_role(node.role());
                    match node.label() {
                        Some(l) => existing.set_label(l),
                        None => existing.clear_label(),
                    }
                    set_flags(&mut existing, &node);
                }
            }
            None => {
                entity_mut.insert(AccessibilityNode::from(node));
            }
        }
    }
}

fn set_flags(target: &mut accesskit::Node, from: &accesskit::Node) {
    match from.toggled() {
        Some(t) => target.set_toggled(t),
        None => target.clear_toggled(),
    }
    match from.is_selected() {
        Some(s) => target.set_selected(s),
        None => target.clear_selected(),
    }
    match from.is_expanded() {
        Some(x) => target.set_expanded(x),
        None => target.clear_expanded(),
    }
    if from.is_disabled() {
        target.set_disabled();
    } else {
        target.clear_disabled();
    }
}
