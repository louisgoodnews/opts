//! Facts about the user's content (catalogue counts, last scan, folder state, texture cache),
//! gathered **off the main thread** (M-0088).
//!
//! Reading them takes real time: the kind counts join over tens of thousands of entries, the
//! change check stats the OMSI folder (often on another disk), the cache statistic lists a
//! folder. Doing that while the *Settings -> Content* page is built froze the application.
//! Now the page asks [`ContentFactsStore`], shows "Calculating..." until the answer arrives and
//! never waits:
//!
//! * [`ContentFactsPlugin`] runs [`drive_content_facts`] every frame; it spawns **one** short-lived
//!   thread per request and reads the result with `try_recv` (a few nanoseconds).
//! * The *light* facts (last scan, totals, folder state) are refreshed every few seconds while a
//!   page that shows them is open; the *heavy* ones (kind counts, cache statistic) only when
//!   [`ContentFactsStore::invalidate`] was called: a scan finished, the cache was cleared, the
//!   OMSI folder changed. Anything else keeps showing the cached value.
//! * The work behind the store is a [`FactsProvider`], replaceable in tests (a slow fake proves
//!   that the page builds within one frame, see `tests/content_page_budget.rs`).

mod disk;
mod poll;
mod store;

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use opts_catalogue::KindCounts;

use crate::catalogue_service::{catalogue_db_path, CatalogueService, DiscoveryStatus};
use crate::settings::GameSettings;

pub use disk::DiskFacts;
pub use store::ContentFactsStore;

/// How often the light facts are refreshed while a page that shows them is open.
pub const LIGHT_REFRESH: Duration = Duration::from_secs(3);

/// What the OMSI folder looks like from here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RootState {
    /// Not set and not found.
    #[default]
    Unset,
    /// Set but not a folder.
    Missing,
    /// There.
    Present,
}

/// Cheap facts, refreshed regularly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LightFacts {
    /// Unix time of the last completed scan.
    pub last_scan: Option<i64>,
    /// Entries the last scan counted.
    pub entities: i64,
    /// The OMSI folder.
    pub root: RootState,
    /// The OMSI folder or a top-level folder in it changed after the last scan.
    pub folder_changed: bool,
}

/// Expensive facts, refreshed only after an invalidation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeavyFacts {
    /// The catalogue by kind.
    pub kinds: KindCounts,
}

/// Everything the Content page and its scan chip show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContentFacts {
    /// Refreshed every few seconds.
    pub light: LightFacts,
    /// Refreshed on invalidation.
    pub heavy: HeavyFacts,
}

/// Where the facts come from.
#[derive(Debug, Clone, Default)]
pub struct FactsRequest {
    /// The catalogue database.
    pub db_path: Option<PathBuf>,
    /// The OMSI folder known to the application (service root, else the setting).
    pub root: Option<PathBuf>,
    /// Read the heavy facts too.
    pub heavy: bool,
}

/// Gathers the facts. Runs on a worker thread; may take as long as it likes.
pub trait FactsProvider: Send + Sync + 'static {
    /// The light facts.
    fn light(&self, req: &FactsRequest) -> LightFacts;
    /// The heavy facts.
    fn heavy(&self, req: &FactsRequest) -> HeavyFacts;
    /// The names of the available theme files (listed with the heavy facts).
    fn theme_files(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Marks a UI element that shows content facts: while one exists, the facts are kept fresh.
#[derive(Component, Debug, Clone, Copy)]
pub struct WantsContentFacts;

/// Delivers finished answers and starts due requests; invalidates on scan completion.
pub fn drive_content_facts(
    time: Res<Time<Real>>,
    mut store: ResMut<ContentFactsStore>,
    settings: Option<Res<GameSettings>>,
    service: Option<Res<CatalogueService>>,
    wanted: Query<(), With<WantsContentFacts>>,
    mut was_running: Local<bool>,
) {
    let running = service
        .as_deref()
        .is_some_and(|s| matches!(s.status(), DiscoveryStatus::Running(_)));
    if *was_running && !running {
        store.invalidate();
    }
    *was_running = running;
    // `bypass_change_detection` keeps `is_changed` meaningful: only a real answer marks it.
    let before = store.generation;
    store.bypass_change_detection().poll();
    if store.generation != before {
        store.set_changed();
    }
    if wanted.is_empty() {
        return;
    }
    let req = FactsRequest {
        db_path: catalogue_db_path(),
        root: service
            .as_deref()
            .map(|s| s.root.clone())
            .or_else(|| settings.and_then(|s| s.omsi_path.clone())),
        heavy: false,
    };
    store
        .bypass_change_detection()
        .maybe_start(time.delta(), req);
}

/// Registers the store and its driver.
#[derive(Default)]
pub struct ContentFactsPlugin;

impl Plugin for ContentFactsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ContentFactsStore>()
            .add_systems(Update, drive_content_facts);
    }
}
