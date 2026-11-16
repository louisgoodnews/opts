//! Home's lifecycle: the remembered drives are read once at startup, the page is despawned on
//! leaving the main menu, and the page is built again when the catalogue arrives.

use bevy::prelude::*;

use crate::drive_history::DriveHistory;
use crate::shell::banner::dismiss_banner;

use super::{HomeDrives, HomeRoot, BANNER_EMPTY, BANNER_NO_OMSI, view};
use crate::new_game_screen::types::GameCatalogue;

/// Reads the remembered drives once at startup (the only writer is Home itself, which keeps the
/// copy in memory current), so building Home never touches the disk.
pub(super) fn load_drive_history(mut drives: ResMut<HomeDrives>) {
    if drives.persist {
        drives.history = DriveHistory::load();
    }
}

/// The catalogue finishes after Home was built (first start): build the page again so the
/// *Continue* card finds its map and picture.
pub(super) fn rebuild_when_content_arrives(
    mut commands: Commands,
    lists: Res<GameCatalogue>,
    roots: Query<Entity, With<HomeRoot>>,
) {
    if lists.is_changed() && !lists.is_added() && !roots.is_empty() {
        for root in &roots {
            commands.entity(root).despawn();
        }
        commands.run_system_cached(view::spawn_home);
    }
}

pub(super) fn despawn_home(mut commands: Commands, roots: Query<Entity, With<HomeRoot>>) {
    for root in &roots {
        commands.entity(root).despawn();
    }
    dismiss_banner(&mut commands, BANNER_NO_OMSI);
    dismiss_banner(&mut commands, BANNER_EMPTY);
}
