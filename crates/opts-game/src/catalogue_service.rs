//! Background discovery and menu lists backed by the SQLite catalogue.
//!
//! On startup the catalogue database is opened and the menus are filled from it immediately
//! (whatever the last scan stored). If the catalogue is missing, built for another root, or
//! older than the configured interval (default 24 h), a discovery task runs on a background
//! thread; when it finishes the menu lists are reloaded.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use opts_catalogue::{Catalogue, Phase, Staleness, Summary};
use opts_gui::components::toast::ToastBuilder;

use crate::app::AppArgs;
use crate::new_game_screen::types::{GameCatalogue, MapItem, NewGameSetup, VehicleItem};
use crate::omsi_root::locate_omsi_root;
use crate::settings::GameSettings;

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
    GameSettings::database_path().map(|p| p.with_file_name("catalogue.db"))
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
    let na = || "N/A".to_string();
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
            description: v.description,
            engine_power: na(),
            transmission: na(),
            capacity: na(),
            length_m: na(),
            max_speed_kmh: na(),
            passenger_doors: na(),
        })
        .collect();
    GameCatalogue { maps, vehicles }
}

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

/// Keeps the current menu selection valid after the lists change.
pub fn reconcile_selection(lists: &GameCatalogue, setup: &mut NewGameSetup) {
    if !lists
        .vehicles
        .iter()
        .any(|v| v.full_name == setup.selected_vehicle)
    {
        if let Some(v) = lists
            .vehicles
            .iter()
            .find(|v| v.id.eq_ignore_ascii_case("MAN_SD77"))
            .or_else(|| lists.vehicles.first())
        {
            setup.selected_vehicle.clone_from(&v.full_name);
        }
    }
    if !lists
        .maps
        .iter()
        .any(|m| m.friendly_name == setup.selected_map)
    {
        if let Some(m) = lists.maps.first() {
            setup.selected_map.clone_from(&m.friendly_name);
        }
    }
}

/// Plugin wiring the catalogue service into the app.
#[derive(Default)]
pub struct CataloguePlugin;

impl Plugin for CataloguePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameCatalogue>()
            .init_resource::<NewGameSetup>()
            .add_systems(Startup, start_catalogue_service)
            .add_systems(Update, poll_catalogue_discovery);
    }
}

/// Starts a discovery scan on a background thread (no-op if one is already running).
pub fn spawn_discovery(service: &CatalogueService) {
    if matches!(service.status(), DiscoveryStatus::Running(_)) {
        return;
    }
    if let Ok(mut s) = service.status.lock() {
        *s = DiscoveryStatus::Running("starting");
    }
    if let Ok(mut n) = service.notified_done.lock() {
        *n = false;
    }
    let (status, root, db_path) = (
        service.status.clone(),
        service.root.clone(),
        service.db_path.clone(),
    );
    let spawned = std::thread::Builder::new()
        .name("catalogue-discovery".into())
        .spawn(move || {
            let set_phase = |p: Phase| {
                if let Ok(mut s) = status.lock() {
                    *s = DiscoveryStatus::Running(match p {
                        Phase::Walking => "scanning files",
                        Phase::Detecting => "reading content",
                        Phase::Writing => "saving",
                    });
                }
            };
            let result = opts_catalogue::discover(&db_path, &root, &set_phase);
            if let Ok(mut s) = status.lock() {
                *s = match result {
                    Ok(summary) => DiscoveryStatus::Done(summary),
                    Err(e) => DiscoveryStatus::Failed(e.to_string()),
                };
            }
        });
    if let Err(e) = spawned {
        error!("cannot start discovery thread: {e}");
    }
}

/// Test support: builds a miniature OMSI installation on disk, runs the real discovery on it
/// and returns the menu lists. Used by integration tests so they never depend on a local
/// OMSI installation.
#[doc(hidden)]
pub mod testing {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{load_menu_lists, Catalogue, GameCatalogue};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn entry(id: u32, code: u32, name: &str) -> String {
        format!("{id}\n{code}\n0\n1.0\n2.0\n3.0\n0.0\n0.0\n0.0\n1.0\n18\n{name}\n")
    }

    /// 42 maps (`Berlin-Spandau`, `Grundorf` with entry points, plus 40 generic ones) and 60
    /// playable vehicles (`MAN_SD77` among them).
    #[must_use]
    pub fn fixture_lists() -> GameCatalogue {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("opts_fixture_{}_{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let spandau = format!(
            "[name]\nBerlin-Spandau\n\n[entrypoints]\n2\n{}{}",
            entry(1, 100, "Spandau Westerwaldstr./Siegener Str."),
            entry(2, 101, "Rathaus Spandau"),
        );
        let grundorf = format!(
            "[name]\nGrundorf\n\n[entrypoints]\n1\n{}",
            entry(1, 200, "Bauernhof")
        );
        for (folder, cfg) in [("Berlin-Spandau", spandau), ("Grundorf", grundorf)] {
            let dir = root.join("maps").join(folder);
            std::fs::create_dir_all(&dir).expect("fixture dir");
            std::fs::write(dir.join("global.cfg"), cfg).expect("fixture cfg");
        }
        for i in 0..40 {
            let dir = root.join("maps").join(format!("Generic_Map_{i:02}"));
            std::fs::create_dir_all(&dir).expect("fixture dir");
            std::fs::write(
                dir.join("global.cfg"),
                format!("[name]\nGeneric Map {i:02}\n"),
            )
            .expect("fixture cfg");
        }
        let mut vehicles: Vec<(String, String, String)> = vec![
            ("MAN_SD77".into(), "MAN SD200".into(), "SD77".into()),
            ("MAN_EN92_main".into(), "MAN".into(), "NL202 - EN92".into()),
        ];
        vehicles.extend((0..58).map(|i| {
            (
                format!("Fleet_{i:02}"),
                "Fleet".into(),
                format!("Model {i:02}"),
            )
        }));
        for (id, maker, model) in vehicles
            .iter()
            .map(|(a, b, c)| (a.as_str(), b.as_str(), c.as_str()))
        {
            let dir = root.join("Vehicles").join(id);
            std::fs::create_dir_all(&dir).expect("fixture dir");
            std::fs::write(
                dir.join(format!("{id}.bus")),
                format!(
                    "[friendlyname]\n{maker}\n{model}\nStandard\n\n[passengercabin]\ncabin.cfg\n\n\
                     [add_camera_driver]\n0.5\n0.3\n1.2\n0\n60\n0\n0\n"
                ),
            )
            .expect("fixture bus");
            std::fs::write(dir.join("cabin.cfg"), "[drivpos]\n0.5\n0.3\n1.0\n0\n")
                .expect("fixture cabin");
        }
        let db = root.join("catalogue.db");
        opts_catalogue::discover(&db, &root, &|_| {}).expect("fixture discovery");
        let lists = load_menu_lists(&Catalogue::open(&db).expect("fixture open"));
        let _ = std::fs::remove_dir_all(&root);
        lists
    }
}
