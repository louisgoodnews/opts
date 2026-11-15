//! Roles, labels and states of UI elements, and the tree snapshot behind `ui.tree`.
//!
//! One place derives what an element *is* from its components, so the control protocol, the
//! accessibility nodes ([`crate::ui_id`]) and the CI rule tests all read the same facts.

use bevy::prelude::*;

mod labels;
mod role;

pub use labels::*;
pub use role::*;

/// Marks decorative text (glyphs, chevrons, icons) that is not part of an element's name.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct A11yHidden;

/// Marks a row of a list (the tree reports it as a list item).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ListRow;

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

mod snapshot;
pub use snapshot::*;
