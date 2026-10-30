//! The formation editor: the *Formation* tab of the Select Vehicle modal (Left Alt, *Spawn
//! vehicle…*, *Formation*). The player composes a train from rail cars (add, remove, move, turn,
//! mark which cars pull), loads a `.zug` consist of the installation as a start, and spawns it at
//! a free entry point. Design: `docs/src/architecture/train-formation-editor.md`.
//!
//! The editing itself is on [`TrainFormation`] (`opts_core::formats::trains`), validation on
//! [`crate::formation`]; this module is state, buttons and the page.

mod actions;
mod build;
mod state;

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use opts_gui::prelude::MaterialTheme;

use super::vehicle_menu::VehicleMenu;
use actions::{formation_actions, load_lists, refresh_validation, search_input};
pub use state::{FormationCommands, FormationEditor};
use state::{FormationList, FormationRoot};

use build::{build_formation, Entries};

/// Registers the editor.
#[derive(Default)]
pub struct FormationMenuPlugin;

impl Plugin for FormationMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FormationEditor>()
            .init_resource::<FormationCommands>()
            .init_resource::<crate::formation::FormationLibrary>()
            .add_systems(
                Update,
                (
                    load_lists,
                    search_input,
                    formation_actions,
                    refresh_validation,
                    rebuild_formation,
                    scroll_lists,
                )
                    .chain()
                    .run_if(in_state(crate::AppState::InGame)),
            )
            .add_systems(OnEnter(crate::AppState::InGame), reset_editor);
    }
}

fn reset_editor(mut ed: ResMut<FormationEditor>) {
    *ed = FormationEditor::default();
}

/// The entry points, the chosen one and the index progress of the last build.
type ShownEntries = Option<(Vec<(u32, String)>, Option<u32>, (usize, bool))>;

type Roots<'w, 's> = Query<'w, 's, Entity, With<FormationRoot>>;

/// Builds the page when it is open and something changed, and removes it when it is not.
fn rebuild_formation(
    mut ed: ResMut<FormationEditor>,
    menu: Res<VehicleMenu>,
    mut commands: Commands,
    roots: Roots,
    scrolls: Query<(&FormationList, &ScrollPosition)>,
    theme: Option<Res<MaterialTheme>>,
    mut shown: Local<ShownEntries>,
) {
    if !menu.formation_open() {
        for r in &roots {
            commands.entity(r).despawn();
        }
        ed.built = None;
        *shown = None;
        return;
    }
    let entries = (
        menu.free_entries().to_vec(),
        menu.entry(),
        menu.entry_progress(),
    );
    if ed.built == Some(ed.revision) && shown.as_ref() == Some(&entries) && !roots.is_empty() {
        return;
    }
    for (l, p) in &scrolls {
        if let Some(s) = ed.scroll.get_mut(l.0) {
            *s = p.y;
        }
    }
    for r in &roots {
        commands.entity(r).despawn();
    }
    if theme.is_none() {
        return;
    }
    ed.built = Some(ed.revision);
    build_formation(
        &mut commands,
        &ed,
        &Entries {
            free: &entries.0,
            chosen: entries.1,
            progress: entries.2,
        },
    );
    *shown = Some(entries);
}

/// Mouse wheel over a list scrolls it.
fn scroll_lists(
    mut wheel: MessageReader<MouseWheel>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut lists: Query<(&mut ScrollPosition, &UiGlobalTransform, &ComputedNode), With<FormationList>>,
) {
    let Ok(window) = window.single() else {
        return;
    };
    let Some(cursor) = window.physical_cursor_position() else {
        wheel.clear();
        return;
    };
    for e in wheel.read() {
        let delta = -e.y * 36.0;
        for (mut pos, transform, node) in &mut lists {
            if !node.contains_point(*transform, cursor) {
                continue;
            }
            let max =
                ((node.content_size().y - node.size().y) * node.inverse_scale_factor()).max(0.0);
            pos.y = (pos.y + delta).clamp(0.0, max);
            break;
        }
    }
}

#[cfg(test)]
mod tests;
