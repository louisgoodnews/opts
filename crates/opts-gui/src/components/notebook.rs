//! Material Design 3 Notebook / Tabbed container widget.

mod builder;
mod spawn;
mod systems;

pub use builder::NotebookBuilder;
pub use systems::{
    notebook_action_system, notebook_focus_selects_system, notebook_sync_system,
    notebook_tab_click_system,
};

use bevy::prelude::*;

use crate::interaction::events::NotebookTabSelected;

/// Marker component for an M3 Notebook tabbed container root entity.
#[derive(Component, Debug, Clone, Default)]
pub struct M3Notebook {
    pub active_tab: usize,
    pub tab_count: usize,
}

/// Marker component for an individual tab header button.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookTabHeader {
    pub notebook: Entity,
    pub index: usize,
}

/// Marker component for the active indicator underline of a tab header.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookTabIndicator {
    pub notebook: Entity,
    pub index: usize,
}

/// Marker component for a content panel associated with a tab.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotebookPanel {
    pub notebook: Entity,
    pub index: usize,
}

/// Plugin registering Notebook widget systems and events.
#[derive(Default)]
pub struct NotebookPlugin;

impl Plugin for NotebookPlugin {
    fn build(&self, app: &mut App) {
        if !app
            .world()
            .contains_resource::<Messages<NotebookTabSelected>>()
        {
            app.add_message::<NotebookTabSelected>();
        }
        app.add_systems(
            Update,
            (
                notebook_tab_click_system,
                notebook_action_system,
                notebook_focus_selects_system,
                notebook_sync_system,
            ),
        );
    }
}

#[cfg(test)]
mod tests;
