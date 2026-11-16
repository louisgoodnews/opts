//! The vocabulary of the control protocol: [`UiRole`] and what element is which role.

use bevy::prelude::*;

use super::ListRow;
use crate::components::{
    checkbox::M3Checkbox,
    combobox::{ComboboxHeader, ComboboxMenuItem},
    modal::ModalRoot,
    notebook::{M3Notebook, NotebookPanel, NotebookTabHeader},
    radio::M3Radio,
    switch::M3Switch,
    text_area::TextArea,
    text_field::{M3TextField, TextFieldInput},
    toast::M3Toast,
    treeview::{M3TreeNode, M3Treeview},
};
use crate::interaction::focus::{FocusRegion, Focusable};

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
