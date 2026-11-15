//! Element state of the tree snapshot (the states and their protocol spellings).

use bevy::prelude::*;

use crate::components::{
    checkbox::M3Checkbox,
    combobox::{ComboboxHeader, M3Combobox},
    notebook::{M3Notebook, NotebookTabHeader},
    radio::M3Radio,
    switch::M3Switch,
    treeview::{M3TreeNode, M3Treeview},
};
use crate::interaction::state::{CheckState, Disabled, DisabledReason, ErrorState, Focused, Pressed, Selected};
use crate::ui_tree::UiNodeState;

mod build;

pub use build::*;

/// The state of one element (visible and not focus-owner by default).
pub fn state_snapshot(world: &World, entity: Entity) -> UiNodeState {
    state_of(world, entity, true, None)
}

pub(super) fn state_of(
    world: &World,
    entity: Entity,
    visible: bool,
    focus: Option<Entity>,
) -> UiNodeState {
    let e = world.entity(entity);
    let proxy_focus = e
        .get::<crate::interaction::layer::FocusProxy>()
        .is_some_and(|p| world.get::<Focused>(p.0).is_some());
    let mut st = UiNodeState {
        hovered: e
            .get::<crate::interaction::state::Hovered>()
            .is_some_and(|h| h.0),
        focused: e.contains::<Focused>() || proxy_focus || focus == Some(entity),
        pressed: e.contains::<Pressed>(),
        visible,
        ..default()
    };
    if e.contains::<Disabled>() {
        st.disabled = Some(
            e.get::<DisabledReason>()
                .map(|r| r.0.to_string())
                .unwrap_or_default(),
        );
    }
    st.error = e.get::<ErrorState>().map(|x| x.0.to_string());
    if let Some(c) = e.get::<M3Checkbox>() {
        st.checked = Some(match c.state {
            CheckState::Checked => "true",
            CheckState::Unchecked => "false",
            CheckState::Indeterminate => "mixed",
        });
    }
    if let Some(s) = e.get::<M3Switch>() {
        st.checked = Some(if s.is_on { "true" } else { "false" });
    }
    if let Some(r) = e.get::<M3Radio>() {
        st.checked = Some(if r.is_selected { "true" } else { "false" });
        st.selected = Some(r.is_selected);
    }
    if let Some(c) = e.get::<M3Combobox>() {
        st.expanded = Some(c.is_open);
    }
    if let Some(h) = e.get::<ComboboxHeader>() {
        st.expanded = world.get::<M3Combobox>(h.0).map(|c| c.is_open);
    }
    if let Some(n) = e.get::<M3TreeNode>() {
        st.expanded = n.has_children.then_some(n.is_expanded);
        st.selected = world
            .get::<M3Treeview>(n.tree_entity)
            .map(|t| t.selected_node_id.as_deref() == Some(n.node_id.as_str()));
    }
    if let Some(t) = e.get::<NotebookTabHeader>() {
        st.selected = world
            .get::<M3Notebook>(t.notebook)
            .map(|nb| nb.active_tab == t.index);
    }
    if e.contains::<Selected>() {
        st.selected = Some(true);
    }
    st
}
