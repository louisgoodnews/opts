//! Stable path ids for every interactive element, and the accessibility nodes derived from them.
//!
//! An id is `parent-id/role:slug`: the parent is the nearest ancestor that has an id, the role is
//! the protocol role ([`crate::ui_tree::UiRole`]) and the slug comes from, in this order, the
//! element's [`L10nKey`](crate::l10n::L10nKey) (language independent), its label text, or its
//! position. Screens that want ids that survive a copy change or a language switch set a
//! [`UiId`] themselves (`menu.start`); the automatic ones are for everything else and are made
//! unique with a numeric suffix.

use std::borrow::Cow;
use std::collections::HashSet;

use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;

use crate::ui_tree::{label_of, role_of, UiRole};

/// The stable id of a UI element.
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiId(pub Cow<'static, str>);

impl UiId {
    /// Creates an explicit id.
    pub fn new(id: impl Into<Cow<'static, str>>) -> Self {
        Self(id.into())
    }
}

/// Lower-case alphanumeric slug of a label, at most 32 characters.
pub fn slug(label: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in label.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
        if out.chars().count() >= 32 {
            break;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Elements that get an automatic id.
type NeedsId = (
    Without<UiId>,
    Without<crate::components::text_field::TextFieldInput>,
    Or<(
        With<crate::interaction::focus::Focusable>,
        With<crate::interaction::focus::FocusRegion>,
        With<crate::components::text_field::M3TextField>,
        With<crate::components::modal::ModalRoot>,
        With<crate::components::toast::M3Toast>,
        With<crate::components::notebook::M3Notebook>,
        With<crate::components::notebook::NotebookPanel>,
        With<crate::components::treeview::M3Treeview>,
    )>,
);

/// Position of an element in the layout tree: root index, then child index per level. Sorting by
/// it gives the order the elements appear in, independent of how entities were allocated.
fn layout_path(world: &World, entity: Entity) -> Vec<u32> {
    let mut path = Vec::new();
    let mut cur = entity;
    while let Some(parent) = world.get::<ChildOf>(cur) {
        let index = world
            .get::<Children>(parent.parent())
            .and_then(|c| c.iter().position(|k| k == cur))
            .unwrap_or(0);
        path.push(index as u32);
        cur = parent.parent();
    }
    path.push(cur.index_u32());
    path.reverse();
    path
}

/// Gives every element without an id one (runs after the frame's spawns).
pub fn assign_ui_ids(world: &mut World) {
    let mut missing: Vec<Entity> = world
        .query_filtered::<Entity, NeedsId>()
        .iter(world)
        .collect();
    if missing.is_empty() {
        return;
    }
    missing.sort_by_cached_key(|e| layout_path(world, *e));
    let mut taken: HashSet<String> = world
        .query::<&UiId>()
        .iter(world)
        .map(|u| u.0.to_string())
        .collect();
    for entity in missing {
        let Ok(e) = world.get_entity(entity) else {
            continue;
        };
        let Some(role) = role_of(&e).or(Some(UiRole::Button)) else {
            continue;
        };
        let mut parent_id = String::new();
        let mut cur = entity;
        while let Some(p) = world.get::<ChildOf>(cur) {
            cur = p.parent();
            if let Some(id) = world.get::<UiId>(cur) {
                parent_id = id.0.to_string();
                break;
            }
        }
        let mut s =
            slug(&crate::l10n::key_of(world, entity).unwrap_or_else(|| label_of(world, entity)));
        if s.is_empty() {
            let index = world
                .get::<ChildOf>(entity)
                .and_then(|p| world.get::<Children>(p.parent()))
                .and_then(|c| c.iter().position(|k| k == entity))
                .unwrap_or(0);
            s = format!("n{index}");
        }
        let base = if parent_id.is_empty() {
            format!("{}:{s}", role.as_str())
        } else {
            format!("{parent_id}/{}:{s}", role.as_str())
        };
        let mut id = base.clone();
        let mut n = 2;
        while taken.contains(&id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        taken.insert(id.clone());
        world.entity_mut(entity).insert(UiId::new(id));
    }
}

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

/// Registers the id and accessibility systems.
pub fn build(app: &mut App) {
    app.add_systems(PostUpdate, (assign_ui_ids, sync_accessibility).chain());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_short_lower_case_and_dash_separated() {
        assert_eq!(slug("Load Last Game"), "load-last-game");
        assert_eq!(slug("  Ärger & Co.  "), "ärger-co");
        assert_eq!(slug(""), "");
        assert!(slug(&"x".repeat(80)).chars().count() <= 32);
    }
}
