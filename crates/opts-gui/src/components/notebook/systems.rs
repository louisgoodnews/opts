//! Systems that switch the active tab of a notebook and keep its visuals in sync.

use bevy::prelude::*;

use super::{M3Notebook, NotebookPanel, NotebookTabHeader, NotebookTabIndicator};
use crate::{
    actions::UiAction,
    interaction::{
        activation::Activated,
        events::NotebookTabSelected,
        state::{Disabled, Focused},
    },
};

/// System handling clicks on tab headers to switch the active tab.
pub fn notebook_tab_click_system(
    mut activated: MessageReader<Activated>,
    header_query: Query<&NotebookTabHeader, Without<Disabled>>,
    mut notebook_query: Query<&mut M3Notebook>,
    mut events: MessageWriter<NotebookTabSelected>,
) {
    for Activated(entity) in activated.read() {
        if let Ok(header) = header_query.get(*entity) {
            if let Ok(mut nb) = notebook_query.get_mut(header.notebook) {
                if nb.active_tab != header.index {
                    nb.active_tab = header.index;
                    events.write(NotebookTabSelected {
                        entity: header.notebook,
                        tab_index: header.index,
                    });
                }
            }
        }
    }
}

/// [`UiAction::NextTab`] / [`UiAction::PrevTab`] switch the notebook that holds focus (else the
/// first one), wrapping around.
pub fn notebook_action_system(
    mut actions: MessageReader<UiAction>,
    focused: Query<&NotebookTabHeader, With<Focused>>,
    mut notebooks: Query<(Entity, &mut M3Notebook)>,
    mut events: MessageWriter<NotebookTabSelected>,
) {
    for action in actions.read() {
        let step: isize = match action {
            UiAction::NextTab => 1,
            UiAction::PrevTab => -1,
            _ => continue,
        };
        let target = focused
            .iter()
            .next()
            .map(|h| h.notebook)
            .or_else(|| notebooks.iter().map(|(e, _)| e).min());
        let Some((entity, mut nb)) = target.and_then(|t| notebooks.get_mut(t).ok()) else {
            continue;
        };
        if nb.tab_count == 0 {
            continue;
        }
        let count = nb.tab_count as isize;
        nb.active_tab = (nb.active_tab as isize + step).rem_euclid(count) as usize;
        events.write(NotebookTabSelected {
            entity,
            tab_index: nb.active_tab,
        });
    }
}

/// Moving focus onto a tab header with the keyboard selects that tab (automatic activation).
pub fn notebook_focus_selects_system(
    headers: Query<&NotebookTabHeader, (Added<Focused>, Without<Disabled>)>,
    mut notebooks: Query<&mut M3Notebook>,
    mut events: MessageWriter<NotebookTabSelected>,
) {
    for header in &headers {
        if let Ok(mut nb) = notebooks.get_mut(header.notebook) {
            if nb.active_tab != header.index {
                nb.active_tab = header.index;
                events.write(NotebookTabSelected {
                    entity: header.notebook,
                    tab_index: header.index,
                });
            }
        }
    }
}

/// System synchronizing tab indicator visibility and panel visibility with the active tab.
pub fn notebook_sync_system(
    nb_query: Query<(Entity, &M3Notebook), Changed<M3Notebook>>,
    mut indicator_query: Query<(&NotebookTabIndicator, &mut Node), Without<NotebookPanel>>,
    mut panel_query: Query<(&NotebookPanel, &mut Node), Without<NotebookTabIndicator>>,
) {
    for (entity, nb) in &nb_query {
        for (indicator, mut style) in &mut indicator_query {
            if indicator.notebook == entity {
                style.display = if indicator.index == nb.active_tab {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }

        for (panel, mut style) in &mut panel_query {
            if panel.notebook == entity {
                style.display = if panel.index == nb.active_tab {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
}
