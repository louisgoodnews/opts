//! Home's inputs and status: what the page depends on (the installation and the catalogue),
//! and the sync that keeps the chip, the drive buttons and the banners in step with it.

use bevy::prelude::*;

use crate::catalogue_service::{CatalogueService, DiscoveryStatus};
use crate::new_game_screen::types::GameCatalogue;
use crate::shell::keys;

use super::model::{CatalogueStatus, DriveBlock, HomeInputs};
use super::params::{HomeFacts, HomeWidgets};
use super::status::{update_banners, update_buttons, update_chips};

/// What Home depends on, read from the world.
fn current_inputs(
    service: Option<&CatalogueService>,
    lists: &GameCatalogue,
    root_found: bool,
) -> HomeInputs {
    let omsi_found = service.is_some() || root_found;
    let stage = service.and_then(|s| match s.status() {
        DiscoveryStatus::Running(stage) => Some(stage),
        _ => None,
    });
    let refreshing = stage.is_some();
    let has_content = !lists.maps.is_empty() && !lists.vehicles.is_empty();
    let catalogue = match service {
        None => CatalogueStatus::Unavailable,
        Some(_) if refreshing && !has_content => CatalogueStatus::Reading,
        Some(_) => CatalogueStatus::Ready {
            maps: lists.maps.len(),
            vehicles: lists.vehicles.len(),
        },
    };
    HomeInputs {
        omsi_found,
        catalogue,
        refreshing,
        stage,
    }
}

pub(super) fn block_reason_key(block: DriveBlock) -> Option<&'static str> {
    match block {
        DriveBlock::None => None,
        DriveBlock::NoOmsi => Some(keys::HOME_REASON_NO_OMSI),
        DriveBlock::Reading => Some(keys::HOME_REASON_READING),
        DriveBlock::Empty => Some(keys::HOME_REASON_EMPTY),
    }
}

/// Keeps the status chip, the drive buttons and the banners in step with the installation.
pub fn sync_home_status(
    mut commands: Commands,
    time: Res<Time>,
    installation: HomeFacts,
    mut since: Local<Option<f32>>,
    mut last: Local<Option<(HomeInputs, String)>>,
    widgets: HomeWidgets,
) {
    let HomeFacts {
        service,
        lists,
        facts,
        loc,
    } = installation;
    let HomeWidgets {
        mut chips,
        buttons,
        fresh,
    } = widgets;
    if !fresh.is_empty() {
        // The page was (re)built: its chip, buttons and banners start from scratch.
        *last = None;
    }
    let elapsed = since.map_or(f32::MAX, |s| s + time.delta_secs());
    *since = Some(elapsed);
    if elapsed < 0.5 && last.is_some() && fresh.is_empty() {
        return;
    }
    *since = Some(0.0);
    // Without a catalogue service the installation is looked for in the background
    // (`content_facts`); until that answer is there nothing is decided (no flicker).
    let root_found = match (service.is_some(), facts.light()) {
        (true, _) => false,
        (false, Some(l)) => l.root == crate::content_facts::RootState::Present,
        (false, None) => return,
    };
    let inputs = current_inputs(service.as_deref(), &lists, root_found);
    let locale = loc.locale().to_owned();
    if last
        .as_ref()
        .is_some_and(|(i, l)| *i == inputs && *l == locale)
    {
        return;
    }
    let previous_banner = last.as_ref().and_then(|(i, _)| i.banner());
    *last = Some((inputs, locale));

    update_chips(&mut commands, &inputs, &mut chips);
    update_buttons(&mut commands, &loc, &inputs, &buttons);
    update_banners(&mut commands, &inputs, previous_banner);
}
