//! Background discovery and menu lists backed by the SQLite catalogue.
//!
//! On startup the catalogue database is opened and the menus are filled from it immediately
//! (whatever the last scan stored). If the catalogue is missing, built for another root, or
//! older than the configured interval (default 24 h), a discovery task runs on a background
//! thread; when it finishes the menu lists are reloaded.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use opts_catalogue::{Catalogue, Phase, Summary};

use opts_new_game_types::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem};

/// State of the background discovery task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryStatus {
    /// Nothing running; the catalogue was current.
    Idle,
    /// A scan is running in the given phase.
    Running(&'static str),
    /// The last scan finished.
    Done(Summary),
    /// The last scan failed.
    Failed(String),
}

/// Handle to the catalogue and the discovery task, shared with systems and loaders.
#[derive(Resource, Clone)]
pub struct CatalogueService {
    /// Query handle.
    pub catalogue: Arc<Catalogue>,
    /// OMSI root the catalogue describes.
    pub root: PathBuf,
    /// Database file.
    pub db_path: PathBuf,
    status: Arc<Mutex<DiscoveryStatus>>,
    notified_done: Arc<Mutex<bool>>,
}

impl CatalogueService {
    /// Current discovery status.
    #[must_use]
    pub fn status(&self) -> DiscoveryStatus {
        self.status
            .lock()
            .map_or(DiscoveryStatus::Idle, |s| s.clone())
    }

    /// Name lookup closure for [`opts_bevy::OmsiInstall::with_name_lookup`].
    #[must_use]
    pub fn name_lookup(&self) -> opts_bevy::install::NameLookup {
        let cat = self.catalogue.clone();
        Arc::new(move |name: &str, near: &str, alt: bool| {
            cat.find_by_name(name, near, alt).ok().flatten()
        })
    }

    /// Relocated-file lookup for [`opts_bevy::OmsiInstall::with_relocate_lookup`].
    #[must_use]
    pub fn relocate_lookup(&self) -> opts_bevy::install::RelocateLookup {
        let cat = self.catalogue.clone();
        Arc::new(move |rel: &str| cat.find_relocated(rel).ok().flatten())
    }

    /// Same-name lookup for [`opts_bevy::OmsiInstall::with_twin_lookup`].
    #[must_use]
    pub fn twin_lookup(&self) -> opts_bevy::install::TwinLookup {
        let cat = self.catalogue.clone();
        Arc::new(move |name: &str| cat.find_all_by_name(name, 64).unwrap_or_default())
    }

    /// Path lookup closure for [`opts_bevy::OmsiInstall::with_lookup`].
    #[must_use]
    pub fn path_lookup(&self) -> opts_bevy::install::PathLookup {
        let cat = self.catalogue.clone();
        Arc::new(move |rel: &str| {
            cat.find_file(rel)
                .ok()
                .flatten()
                .or_else(|| cat.find_dir(rel).ok().flatten())
        })
    }
}

/// Location of the catalogue database (next to the settings database).
#[must_use]
pub fn catalogue_db_path() -> Option<PathBuf> {
    opts_settings_io::database_path().map(|p| p.with_file_name("catalogue.db"))
}

/// Converts catalogue rows into the menu's list types.
#[must_use]
pub fn load_menu_lists(cat: &Catalogue) -> GameCatalogue {
    let maps = cat
        .maps()
        .unwrap_or_default()
        .into_iter()
        .map(|m| MapItem {
            id: m.folder.clone(),
            name: m.folder,
            friendly_name: m.friendly_name,
            description: m.description,
            tile_count: usize::try_from(m.tile_count).unwrap_or(0),
            standard_depot: m.standard_depot,
            years: m.years,
            entry_points: m.entry_points,
        })
        .collect();
    let vehicles = cat
        .playable_vehicles()
        .unwrap_or_default()
        .into_iter()
        .map(|v| VehicleItem {
            full_name: if v.manufacturer.is_empty() {
                v.model.clone()
            } else {
                format!("{} {}", v.manufacturer, v.model)
            },
            id: v.id,
            manufacturer: v.manufacturer,
            model: v.model,
            default_livery: v.livery,
            bus_file: v.marker_file,
            description: v.description,
            class: v.class.unwrap_or_default(),
            mass_tonnes: v.mass_tonnes,
        })
        .collect();
    GameCatalogue { maps, vehicles }
}

mod startup;
pub use startup::{poll_catalogue_discovery, start_catalogue_service};

mod discovery;
pub use discovery::{reconcile_selection, spawn_discovery, CataloguePlugin};

/// Test support: builds a miniature OMSI installation on disk, runs the real discovery on it
/// and returns the menu lists. Used by integration tests so they never depend on a local
/// OMSI installation.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub mod testing;
