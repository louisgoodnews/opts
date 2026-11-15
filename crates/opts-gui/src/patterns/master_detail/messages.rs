//! The messages sent by the pattern and the marker components of its entities.

use bevy::prelude::*;

/// The selection changed (also sent once when the pattern first shows a selection).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailSelected {
    /// The pattern root.
    pub root: Entity,
    /// The entry id; `None` when the selection was cleared.
    pub id: Option<String>,
}

/// The user chose the selected entry (Enter, or a second click on it).
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailChosen {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
}

/// The user starred or un-starred an entry; the pattern has already updated its model, the screen
/// persists the change.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct MasterDetailFavourite {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
    /// The new state.
    pub favourite: bool,
}

/// A thumbnail slot of a row: the screen's thumbnail system fills it (matched by `key`).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RowThumb {
    /// The pattern root.
    pub root: Entity,
    /// The thumbnail key of the entry.
    pub key: String,
}

/// Marks the list node (the focus stop) and names its pattern root.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterList(pub Entity);

/// Marks the row button of an entry.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterRowButton {
    /// The pattern root.
    pub root: Entity,
    /// Index into the flattened rows.
    pub row: usize,
    /// The entry id.
    pub id: String,
}

/// Marks the star of a row.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterStarButton {
    /// The pattern root.
    pub root: Entity,
    /// The entry id.
    pub id: String,
}

/// Marks a filter chip.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct MasterChipButton {
    /// The pattern root.
    pub root: Entity,
    /// The tag it toggles.
    pub tag: String,
}

/// Marks the *Clear filter* button.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterClearButton(pub Entity);

/// Marks the scroll bar thumb.
#[derive(Component, Debug, Clone, Copy)]
pub struct MasterScrollThumb(pub Entity);

/// Marks the root of a pattern.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct MasterDetailRoot;
