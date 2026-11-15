//! The rows of the save list.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use opts_gui::l10n::{L10nArgs, L10nKey, Localization};
use opts_gui::prelude::*;

use super::{RowAction, SavesButton, SavesList, SavesMode, SavesUi};
use crate::in_game::omsi_world::WorldLoadJob;
use crate::pause::stats::format_duration;
use crate::save::container::SaveFile;
use crate::save::registry::{preflight, CheckEnv, Issue, IssueKind, SaveRegistry};
use crate::save::store::SaveEntry;
use crate::save::SaveStorage;
use crate::shell::keys;

mod buttons;
mod details;

use buttons::spawn_row;
use details::line;

/// What the rows are worked out from.
#[derive(SystemParam)]
pub struct SaveListFacts<'w> {
    storage: Res<'w, SaveStorage>,
    registry: Res<'w, SaveRegistry>,
    job: Option<Res<'w, WorldLoadJob>>,
    state: Res<'w, State<crate::app_state::AppState>>,
}

/// Rebuilds the rows when the list changed.
pub fn rebuild_list(
    mut commands: Commands,
    mut ui: ResMut<SavesUi>,
    facts: SaveListFacts,
    loc: Res<Localization>,
    lists: Query<Entity, With<SavesList>>,
) {
    let SaveListFacts {
        storage,
        registry,
        job,
        state,
    } = facts;
    if !ui.dirty {
        return;
    }
    let Ok(list) = lists.single() else { return };
    ui.dirty = false;
    let entries = storage.0.as_ref().map(|s| s.list()).unwrap_or_default();
    let root = job
        .map(|j| j.omsi_root().to_path_buf())
        .or_else(crate::omsi_root::known_omsi_root);
    let in_session = *state.get() == crate::app_state::AppState::InGame;
    let mode = ui.mode;
    commands.entity(list).despawn_children();
    commands.entity(list).with_children(|rows| {
        if entries.is_empty() {
            line(
                rows,
                keys::SAVES_EMPTY,
                Vec::new(),
                TypeScale::BodyMedium,
                ColorRole::OnSurfaceVariant,
            );
        }
        for entry in &entries {
            let issues = entry
                .state
                .as_ref()
                .map(|l| issues_of(&registry, &l.file, root.as_deref()))
                .unwrap_or_default();
            spawn_row(rows, entry, mode, &issues, &loc, in_session);
        }
    });
}

fn issues_of(
    registry: &SaveRegistry,
    file: &SaveFile,
    root: Option<&std::path::Path>,
) -> Vec<Issue> {
    preflight(
        registry.all(),
        file,
        &CheckEnv {
            root,
            fingerprint: &file.fingerprint,
        },
    )
}
