//! Roles, labels and states of UI elements, and the tree snapshot behind `ui.tree`.
//!
//! One place derives what an element *is* from its components, so the control protocol, the
//! accessibility nodes ([`crate::ui_id`]) and the CI rule tests all read the same facts.

use std::collections::HashSet;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::components::{
    checkbox::M3Checkbox,
    combobox::{ComboboxHeader, ComboboxMenuItem, M3Combobox},
    modal::ModalRoot,
    notebook::{M3Notebook, NotebookPanel, NotebookTabHeader},
    radio::M3Radio,
    switch::M3Switch,
    text_area::TextArea,
    text_field::{M3TextField, TextFieldInput},
    toast::M3Toast,
    treeview::{M3TreeNode, M3Treeview},
};
use crate::interaction::{
    focus::{FocusRegion, Focusable},
    navigation::FocusGraph,
    state::{CheckState, Disabled, DisabledReason, ErrorState, Focused, Pressed, Selected},
    tooltip::Tooltip,
};
use crate::ui_id::UiId;

/// Marks decorative text (glyphs, chevrons, icons) that is not part of an element's name.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct A11yHidden;

/// Marks a row of a list (the tree reports it as a list item).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ListRow;

/// What an element is, in the vocabulary of the control protocol (ARIA-like).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UiRole {
    /// Push button.
    Button,
    /// Check box.
    Checkbox,
    /// Radio button.
    Radio,
    /// On/off switch.
    Switch,
    /// Single or multi-line text entry.
    Textbox,
    /// Drop-down header.
    Combobox,
    /// Entry of a drop-down list.
    Option,
    /// Tab strip.
    Tablist,
    /// One tab.
    Tab,
    /// Content of one tab.
    Tabpanel,
    /// Tree view.
    Tree,
    /// Row of a tree.
    Treeitem,
    /// Row of a list.
    Listitem,
    /// Modal dialog.
    Dialog,
    /// Transient message.
    Toast,
    /// Landmark region.
    Region,
}

impl UiRole {
    /// The protocol spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Switch => "switch",
            Self::Textbox => "textbox",
            Self::Combobox => "combobox",
            Self::Option => "option",
            Self::Tablist => "tablist",
            Self::Tab => "tab",
            Self::Tabpanel => "tabpanel",
            Self::Tree => "tree",
            Self::Treeitem => "treeitem",
            Self::Listitem => "listitem",
            Self::Dialog => "dialog",
            Self::Toast => "toast",
            Self::Region => "region",
        }
    }
}

/// The role of an entity, or `None` when it is not a UI element the tree reports.
pub fn role_of(e: &EntityRef) -> Option<UiRole> {
    use UiRole::*;
    Some(if e.contains::<ModalRoot>() {
        Dialog
    } else if e.contains::<M3Toast>() {
        Toast
    } else if e.contains::<M3Notebook>() {
        Tablist
    } else if e.contains::<NotebookTabHeader>() {
        Tab
    } else if e.contains::<NotebookPanel>() {
        Tabpanel
    } else if e.contains::<M3Treeview>() {
        Tree
    } else if e.contains::<M3TreeNode>() {
        Treeitem
    } else if e.contains::<ComboboxMenuItem>() {
        Option
    } else if e.contains::<ComboboxHeader>() {
        Combobox
    } else if e.contains::<M3Checkbox>() {
        Checkbox
    } else if e.contains::<M3Radio>() {
        Radio
    } else if e.contains::<M3Switch>() {
        Switch
    } else if e.contains::<M3TextField>() || e.contains::<TextArea>() {
        Textbox
    } else if e.contains::<ListRow>() {
        Listitem
    } else if e.contains::<FocusRegion>() {
        Region
    } else if e.contains::<Focusable>() && !e.contains::<TextFieldInput>() {
        Button
    } else {
        return None;
    })
}

/// Visible text below `entity` (depth-first, joined by one space).
pub fn text_below(world: &World, entity: Entity) -> String {
    let mut parts = Vec::new();
    collect_text(world, entity, &mut parts, 0);
    parts.join(" ")
}

fn collect_text(world: &World, entity: Entity, out: &mut Vec<String>, depth: usize) {
    if depth > 6 {
        return;
    }
    let Ok(e) = world.get_entity(entity) else {
        return;
    };
    let icon_font = e.get::<TextFont>().is_some_and(
        |f| matches!(&f.font, bevy::text::FontSource::Handle(h) if *h == crate::fonts::ICON_FONT),
    );
    if e.contains::<A11yHidden>() || icon_font {
        return;
    }
    if let Some(text) = e.get::<Text>() {
        if !text.0.trim().is_empty() {
            out.push(text.0.trim().to_string());
        }
    }
    if let Some(children) = e.get::<Children>() {
        for child in children.iter() {
            collect_text(world, child, out, depth + 1);
        }
    }
}

/// The accessible name of an element.
pub fn label_of(world: &World, entity: Entity) -> String {
    let Ok(e) = world.get_entity(entity) else {
        return String::new();
    };
    if let Some(field) = e.get::<M3TextField>() {
        return if field.label.is_empty() {
            field.placeholder.clone()
        } else {
            field.label.clone()
        };
    }
    if let Some(area) = e.get::<TextArea>() {
        return area.placeholder.clone();
    }
    if let Some(toast) = e.get::<M3Toast>() {
        return toast.message.clone();
    }
    let text = text_below(world, entity);
    if text.is_empty() {
        e.get::<Tooltip>()
            .map(|t| t.0.to_string())
            .unwrap_or_default()
    } else {
        text
    }
}

/// Everything a test or an agent may want to know about one element's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiNodeState {
    /// Pointer over the element.
    pub hovered: bool,
    /// Holds (or, for a text field, contains) keyboard focus.
    pub focused: bool,
    /// Pointer button held on it.
    pub pressed: bool,
    /// Selected (tab, list row, tree row, radio).
    pub selected: Option<bool>,
    /// `Some("true" | "false" | "mixed")` for checkable elements (check box, switch, radio).
    pub checked: Option<&'static str>,
    /// Open or expanded (combobox, tree row).
    pub expanded: Option<bool>,
    /// `Some(reason)` when disabled; the reason may be empty (which is a rule violation).
    pub disabled: Option<String>,
    /// Validation message.
    pub error: Option<String>,
    /// Displayed and not hidden by an ancestor.
    pub visible: bool,
}

/// One element of the tree.
#[derive(Debug, Clone, PartialEq)]
pub struct UiNode {
    /// Stable path id.
    pub id: String,
    /// What it is.
    pub role: UiRole,
    /// Accessible name.
    pub label: String,
    /// Value of a text entry.
    pub value: Option<String>,
    /// State.
    pub state: UiNodeState,
    /// `[x, y, width, height]` in logical pixels, when the layout has run.
    pub rect: Option<[f32; 4]>,
    /// Whether the element can take focus right now.
    pub focusable: bool,
    /// Nested elements.
    pub children: Vec<UiNode>,
}

/// The whole tree plus which element has focus.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiSnapshot {
    /// Top-level elements.
    pub roots: Vec<UiNode>,
    /// Id of the focused element, if it has one.
    pub focus: Option<String>,
}

impl UiSnapshot {
    /// All nodes, depth-first.
    pub fn flatten(&self) -> Vec<&UiNode> {
        fn walk<'a>(n: &'a UiNode, out: &mut Vec<&'a UiNode>) {
            out.push(n);
            n.children.iter().for_each(|c| walk(c, out));
        }
        let mut out = Vec::new();
        self.roots.iter().for_each(|r| walk(r, &mut out));
        out
    }

    /// The node with this id.
    pub fn find(&self, id: &str) -> Option<&UiNode> {
        self.flatten().into_iter().find(|n| n.id == id)
    }
}

/// The state of one element (visible and not focus-owner by default).
pub fn state_snapshot(world: &World, entity: Entity) -> UiNodeState {
    state_of(world, entity, true, None)
}

fn state_of(world: &World, entity: Entity, visible: bool, focus: Option<Entity>) -> UiNodeState {
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

fn value_of(world: &World, entity: Entity) -> Option<String> {
    let e = world.entity(entity);
    e.get::<M3TextField>()
        .map(|f| f.text.clone())
        .or_else(|| e.get::<TextArea>().map(|a| a.buffer.text().to_string()))
        .or_else(|| {
            e.get::<M3Combobox>()
                .and_then(|c| c.selected.and_then(|i| c.options.get(i).cloned()))
        })
}

pub(crate) fn rect_of(world: &World, entity: Entity) -> Option<[f32; 4]> {
    let computed = world.get::<ComputedNode>(entity)?;
    let transform = world.get::<UiGlobalTransform>(entity)?;
    let size = computed.size();
    if size.x <= 0.0 || size.y <= 0.0 {
        return None;
    }
    let k = computed.inverse_scale_factor;
    let top_left = (transform.translation - size / 2.0) * k;
    Some([top_left.x, top_left.y, size.x * k, size.y * k])
}

fn build(
    world: &World,
    entity: Entity,
    can_focus: &HashSet<Entity>,
    focus: Option<Entity>,
    visible: bool,
    out: &mut Vec<UiNode>,
) {
    let e = world.entity(entity);
    let visible = visible
        && e.get::<Node>().is_none_or(|n| n.display != Display::None)
        && e.get::<Visibility>() != Some(&Visibility::Hidden);
    let mut children = Vec::new();
    let mut target = &mut *out;
    let own;
    if let (Some(id), Some(role)) = (e.get::<UiId>(), role_of(&e)) {
        own = UiNode {
            id: id.0.to_string(),
            role,
            label: label_of(world, entity),
            value: value_of(world, entity),
            state: state_of(world, entity, visible, focus),
            rect: rect_of(world, entity),
            focusable: can_focus.contains(&entity)
                || e.get::<crate::interaction::layer::FocusProxy>()
                    .is_some_and(|p| can_focus.contains(&p.0)),
            children: Vec::new(),
        };
        target = &mut children;
        let kids: Vec<Entity> = e
            .get::<Children>()
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        for k in kids {
            build(world, k, can_focus, focus, visible, target);
        }
        let mut node = own;
        node.children = children;
        out.push(node);
        return;
    }
    let kids: Vec<Entity> = e
        .get::<Children>()
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    for k in kids {
        build(world, k, can_focus, focus, visible, target);
    }
}

/// Builds the snapshot of everything that carries a [`UiId`].
pub fn snapshot(world: &mut World) -> UiSnapshot {
    let can_focus: HashSet<Entity> = world
        .run_system_once(|graph: FocusGraph| {
            graph
                .candidates()
                .into_iter()
                .map(|c| c.entity)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let focus = world
        .query_filtered::<Entity, With<Focused>>()
        .iter(world)
        .next();
    let mut root_entities: Vec<Entity> = world
        .query_filtered::<Entity, (With<Node>, Without<ChildOf>)>()
        .iter(world)
        .collect();
    root_entities.sort();
    let mut roots = Vec::new();
    for root in root_entities {
        build(world, root, &can_focus, focus, true, &mut roots);
    }
    let focus_id = focus.and_then(|f| {
        let owner = world.get::<TextFieldInput>(f).map_or(f, |input| input.0);
        world.get::<UiId>(owner).map(|id| id.0.to_string())
    });
    UiSnapshot {
        roots,
        focus: focus_id,
    }
}

/// The element for an id, and what a click on it should act on.
pub fn find_entity(world: &mut World, id: &str) -> Option<Entity> {
    world
        .query::<(Entity, &UiId)>()
        .iter(world)
        .find(|(_, u)| u.0 == id)
        .map(|(e, _)| e)
}
