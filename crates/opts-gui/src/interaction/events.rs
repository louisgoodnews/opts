//! Semantic UI events emitted by interactive Material Design 3 widgets.

use crate::interaction::state::CheckState;
use bevy::prelude::*;

/// Event fired when an [`M3Button`](crate::components::button::M3Button) is clicked.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonClicked(pub Entity);

/// Event fired when an [`M3Checkbox`](crate::components::checkbox::M3Checkbox) changes state.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckboxToggled {
    pub entity: Entity,
    pub state: CheckState,
}

/// Event fired when an [`M3Switch`](crate::components::switch::M3Switch) is toggled on or off.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchToggled {
    pub entity: Entity,
    pub is_on: bool,
}

/// Event fired when an [`M3Radio`](crate::components::radio::M3Radio) button is selected.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadioSelected {
    pub entity: Entity,
}

/// Event fired when the text inside an [`M3TextField`](crate::components::text_field::M3TextField) changes.
#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct TextChanged {
    pub entity: Entity,
    pub text: String,
}

/// Event fired when a date is selected via [`M3DateSelect`](crate::components::date_select::M3DateSelect).
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateSelected {
    pub entity: Entity,
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// Event fired when a date and time is selected via [`M3DateTimeSelect`](crate::components::datetime_select::M3DateTimeSelect).
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeSelected {
    pub entity: Entity,
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

/// Event fired when an option is selected in an [`M3Combobox`](crate::components::combobox::M3Combobox).
#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct ComboboxSelected {
    pub entity: Entity,
    pub selected_index: usize,
    pub selected_value: String,
}

/// Event fired when the active tab in an [`M3Notebook`](crate::components::notebook::M3Notebook) changes.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookTabSelected {
    pub entity: Entity,
    pub tab_index: usize,
}

/// Event fired when an [`M3Toast`](crate::components::toast::M3Toast) is dismissed or expires.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastDismissed {
    pub entity: Entity,
}

/// Event fired when an [`M3Toplevel`](crate::components::toplevel::M3Toplevel) window/modal is closed.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToplevelClosed {
    pub entity: Entity,
}

/// Event fired when a node in an [`M3Treeview`](crate::components::treeview::M3Treeview) is expanded or collapsed.
#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct TreeNodeToggled {
    pub tree_entity: Entity,
    pub node_id: String,
    pub is_expanded: bool,
}

/// Event fired when a node in an [`M3Treeview`](crate::components::treeview::M3Treeview) is selected.
#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct TreeNodeSelected {
    pub tree_entity: Entity,
    pub node_id: String,
}

/// Event fired when the active theme mode is changed.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemeChanged {
    pub is_dark: bool,
}
