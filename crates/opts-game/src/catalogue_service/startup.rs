//! Startup of the catalogue service: the catalogue is opened and the menus are filled from it,
//! and discovery starts when the catalogue is stale; the discovery poll reloads the menus.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use opts_catalogue::{Catalogue, Staleness};
use opts_gui::components::toast::ToastBuilder;

use crate::app::AppArgs;
use crate::new_game_screen::types::{GameCatalogue, NewGameSetup};
use crate::omsi_root::locate_omsi_root;
use crate::settings::GameSettings;

use super::{
    catalogue_db_path, load_menu_lists, reconcile_selection, spawn_discovery, CatalogueService,
    DiscoveryStatus,
};

/// Startup: open the catalogue, fill the menus from it, and start discovery if it is stale.
pub fn start_catalogue_service(
    mut commands: Commands,
    settings: Res<GameSettings>,
    args: Option<Res<AppArgs>>,
    mut lists: ResMut<GameCatalogue>,
    mut setup: ResMut<NewGameSetup>,
) {
    let explicit = args
        .as_ref()
        .and_then(|a| a.0.omsi_path.clone())
        .or_else(|| {
            args.as_ref()
                .and_then(|a| a.0.map_path.as_deref())
                .and_then(opts_bevy::OmsiInstall::root_from_map_dir)
        });
    let Some(root) = locate_omsi_root(explicit.as_deref()) else {
        warn!(
            "OMSI installation not found (pass --omsi-path or set $OMSI_PATH); catalogue is empty"
        );
        return;
    };
    let Some(db_path) = catalogue_db_path() else {
        warn!("no config directory available; catalogue disabled");
        return;
    };
    let catalogue = match Catalogue::open(&db_path) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            error!("cannot open catalogue database {}: {e}", db_path.display());
            return;
        }
    };

    *lists = load_menu_lists(&catalogue);
    reconcile_selection(&lists, &mut setup);
    info!(
        path = %db_path.display(),
        maps = lists.maps.len(),
        vehicles = lists.vehicles.len(),
        "menu lists loaded from the SQLite catalogue"
    );

    let max_age =
        Duration::from_secs(u64::from(settings.discovery_interval_days.max(1)) * 24 * 3600);
    let staleness = catalogue
        .staleness(&root, max_age)
        .unwrap_or(Staleness::Missing);
    let service = CatalogueService {
        catalogue,
        root: root.clone(),
        db_path: db_path.clone(),
        status: Arc::new(Mutex::new(DiscoveryStatus::Idle)),
        notified_done: Arc::new(Mutex::new(false)),
    };

    if staleness.needs_scan() {
        info!(?staleness, root = %root.display(), "catalogue is stale; starting background discovery");
        spawn_discovery(&service);
    } else {
        info!(?staleness, "catalogue is current; no discovery needed");
    }
    commands.insert_resource(service);
}

/// Polls the discovery task; on completion reloads the menu lists and notifies the user.
pub fn poll_catalogue_discovery(
    mut commands: Commands,
    service: Option<Res<CatalogueService>>,
    mut lists: ResMut<GameCatalogue>,
    mut setup: ResMut<NewGameSetup>,
) {
    let Some(service) = service else { return };
    let status = service.status();
    let Ok(mut notified) = service.notified_done.lock() else {
        return;
    };
    if *notified {
        return;
    }
    match status {
        DiscoveryStatus::Done(summary) => {
            *notified = true;
            *lists = load_menu_lists(&service.catalogue);
            reconcile_selection(&lists, &mut setup);
            info!(
                ?summary,
                maps = lists.maps.len(),
                vehicles = lists.vehicles.len(),
                "catalogue discovery finished; menu lists reloaded"
            );
            ToastBuilder::new(format!(
                "OMSI catalogue updated: {} maps, {} vehicles",
                lists.maps.len(),
                lists.vehicles.len()
            ))
            .with_duration(4.0)
            .spawn(&mut commands);
        }
        DiscoveryStatus::Failed(e) => {
            *notified = true;
            error!("catalogue discovery failed: {e}");
        }
        DiscoveryStatus::Idle | DiscoveryStatus::Running(_) => {}
    }
}
