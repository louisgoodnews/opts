//! State of the formation editor: the resource, its buttons' actions and what the page shows.

use bevy::prelude::*;
mod editor;
mod lists;

pub use editor::FormationEditor;

/// Every entity of the editor page.
#[derive(Component)]
pub(super) struct FormationRoot;

/// A scrollable list of the page; the number says which carried-over offset it uses.
#[derive(Component)]
pub(super) struct FormationList(pub(super) usize);

/// Commands for the editor from outside the UI (`menu.do formation <command>`): `add <text>`,
/// `trains`/`cars` (the list), `preset <text>`, `search <text>`, `remove|up|down|turn|power <n>`
/// (cars counted from 1), `reverse`, `clear`, `entry <text>`, `spawn`.
#[derive(Resource, Default)]
pub struct FormationCommands(pub Vec<String>);

/// Which list the left column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ListKind {
    /// The rail vehicles of the installation.
    #[default]
    Cars,
    /// The `.zug` consists of the installation.
    Presets,
    /// The formations the user saved.
    Saved,
}

/// What a button of the page does.
#[derive(Component, Clone, Debug)]
pub(super) enum Act {
    /// The active tab.
    Noop,
    /// Back to the Select Vehicle page.
    Back,
    List(ListKind),
    Pick(usize),
    PickPreset(usize),
    /// Loads a saved formation.
    PickSaved(usize),
    /// Deletes a saved formation.
    DeleteSaved(usize),
    /// Saves the train.
    Save,
    /// Typing edits the name instead of the search.
    Rename,
    Add,
    /// Adds the car with this path (the remote `add`).
    AddFile(String),
    Row(usize),
    Up(usize),
    Down(usize),
    Turn(usize),
    Power(usize),
    Remove(usize),
    Reverse,
    Clear,
    Spawn,
    Entry(u32),
}

/// A rail vehicle file of the installation.
#[derive(Debug, Clone)]
pub(super) struct Car {
    /// Path relative to the OMSI root.
    pub(super) rel: String,
    /// What the list shows.
    pub(super) label: String,
    pub(super) mass_t: Option<f64>,
}

/// Numbers shown for the formation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct Summary {
    pub(super) parts: usize,
    pub(super) mass_t: f64,
}
