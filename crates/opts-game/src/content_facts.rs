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

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;
use opts_catalogue::KindCounts;

use crate::catalogue_service::{catalogue_db_path, CatalogueService, DiscoveryStatus};
use crate::settings::GameSettings;

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
    /// Files in the texture cache.
    pub cache_files: usize,
    /// Bytes in the texture cache.
    pub cache_bytes: u64,
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

/// The real provider: the catalogue file, the OMSI folder and the texture cache folder.
#[derive(Debug, Default, Clone, Copy)]
pub struct DiskFacts;

impl FactsProvider for DiskFacts {
    fn light(&self, req: &FactsRequest) -> LightFacts {
        let root = req
            .root
            .clone()
            .or_else(crate::omsi_root::known_omsi_root);
        let root_state = match &root {
            None => RootState::Unset,
            Some(r) if r.is_dir() => RootState::Present,
            Some(_) => RootState::Missing,
        };
        let (last_scan, entities) = open(req).map_or((None, 0), |c| {
            (c.last_scan().ok().flatten(), c.totals().map_or(0, |t| t.1))
        });
        let folder_changed = matches!((&root, last_scan), (Some(r), Some(t)) if r.is_dir()
            && crate::settings_screen::scan_status::folder_changed_since(r, t));
        LightFacts {
            last_scan,
            entities,
            root: root_state,
            folder_changed,
        }
    }

    fn theme_files(&self) -> Vec<String> {
        crate::theme_files::discover_theme_files()
    }

    fn heavy(&self, req: &FactsRequest) -> HeavyFacts {
        let kinds = open(req)
            .and_then(|c| c.kind_counts().ok())
            .unwrap_or_default();
        let (cache_files, cache_bytes) =
            opts_bevy::descramble_cache::directory().map_or((0, 0), |d| dir_usage(&d));
        HeavyFacts {
            kinds,
            cache_files,
            cache_bytes,
        }
    }
}

/// A read-only handle: no file is created, no schema migrated, nothing locks the scan's writes.
fn open(req: &FactsRequest) -> Option<opts_catalogue::Catalogue> {
    req.db_path
        .as_deref()
        .filter(|p| p.exists())
        .and_then(|p| opts_catalogue::Catalogue::open_read_only(p).ok())
}

/// Files and bytes directly in `dir`.
fn dir_usage(dir: &std::path::Path) -> (usize, u64) {
    let mut n = (0, 0);
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            if let Ok(m) = e.metadata() {
                if m.is_file() {
                    n.0 += 1;
                    n.1 += m.len();
                }
            }
        }
    }
    n
}

/// What a worker sends back: the light facts and, when asked, the heavy ones with the theme list.
type Answer = (LightFacts, Option<(HeavyFacts, Vec<String>)>);

/// The cached facts and the request in flight.
#[derive(Resource)]
pub struct ContentFactsStore {
    provider: Arc<dyn FactsProvider>,
    current: Option<ContentFacts>,
    light_ready: bool,
    heavy_ready: bool,
    heavy_stale: bool,
    pending: Option<(Mutex<Receiver<Answer>>, bool)>,
    jobs: Vec<Mutex<Receiver<()>>>,
    themes: Vec<String>,
    since_light: Duration,
    /// Increments whenever new facts arrived (for change detection by readers).
    pub generation: u64,
}

impl Default for ContentFactsStore {
    fn default() -> Self {
        Self::new(Arc::new(DiskFacts))
    }
}

impl ContentFactsStore {
    /// A store over `provider`.
    #[must_use]
    pub fn new(provider: Arc<dyn FactsProvider>) -> Self {
        Self {
            provider,
            current: None,
            light_ready: false,
            heavy_ready: false,
            heavy_stale: true,
            pending: None,
            jobs: Vec::new(),
            themes: Vec::new(),
            since_light: LIGHT_REFRESH,
            generation: 0,
        }
    }

    /// The light facts, once known.
    #[must_use]
    pub fn light(&self) -> Option<LightFacts> {
        self.light_ready.then(|| self.current.unwrap_or_default().light)
    }

    /// The heavy facts, once known (a stale value stays visible until the new one arrives).
    #[must_use]
    pub fn heavy(&self) -> Option<HeavyFacts> {
        self.heavy_ready.then(|| self.current.unwrap_or_default().heavy)
    }

    /// The theme files found at the last heavy refresh (empty until then).
    #[must_use]
    pub fn theme_files(&self) -> &[String] {
        &self.themes
    }

    /// The heavy facts changed (a scan finished, the cache was cleared, the folder changed):
    /// recompute them in the background. The old values stay until the new ones are there.
    pub fn invalidate(&mut self) {
        self.heavy_stale = true;
        self.since_light = LIGHT_REFRESH;
    }

    /// Runs `job` (file-system work such as clearing the texture cache) on a worker thread and
    /// invalidates the heavy facts when it is done.
    pub fn run_then_invalidate(&mut self, job: impl FnOnce() + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("content-job".into())
            .spawn(move || {
                job();
                let _ = tx.send(());
            });
        match spawned {
            Ok(_) => self.jobs.push(Mutex::new(rx)),
            Err(e) => tracing::warn!(err = %e, "cannot start the content job thread"),
        }
    }

    /// A request is running.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Takes a finished answer, if any. Never blocks.
    fn poll(&mut self) {
        let before = self.jobs.len();
        self.jobs.retain(|rx| {
            rx.lock()
                .is_ok_and(|rx| matches!(rx.try_recv(), Err(TryRecvError::Empty)))
        });
        if self.jobs.len() != before {
            self.invalidate();
        }
        let Some((rx, heavy)) = &self.pending else {
            return;
        };
        let heavy_asked = *heavy;
        let received = rx.lock().map_or(Err(TryRecvError::Disconnected), |rx| rx.try_recv());
        match received {
            Ok((light, h)) => {
                let mut f = self.current.unwrap_or_default();
                f.light = light;
                self.light_ready = true;
                if let Some((h, themes)) = h {
                    f.heavy = h;
                    self.themes = themes;
                    self.heavy_ready = true;
                } else if heavy_asked {
                    self.heavy_stale = true;
                }
                self.current = Some(f);
                self.generation += 1;
                self.pending = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                // A worker that died leaves the placeholders; try again at the next refresh.
                self.heavy_stale |= heavy_asked;
            }
        }
    }

    /// Starts a request on a worker thread when one is due.
    fn maybe_start(&mut self, elapsed: Duration, req: FactsRequest) {
        self.since_light += elapsed;
        if self.pending.is_some() {
            return;
        }
        let heavy = self.heavy_stale || !self.heavy_ready;
        if !heavy && self.since_light < LIGHT_REFRESH {
            return;
        }
        self.since_light = Duration::ZERO;
        self.heavy_stale = false;
        let (tx, rx) = mpsc::channel();
        let provider = self.provider.clone();
        let req = FactsRequest { heavy, ..req };
        let spawned = std::thread::Builder::new()
            .name("content-facts".into())
            .spawn(move || {
                let light = provider.light(&req);
                let h = req
                    .heavy
                    .then(|| (provider.heavy(&req), provider.theme_files()));
                let _ = tx.send((light, h));
            });
        match spawned {
            Ok(_) => self.pending = Some((Mutex::new(rx), heavy)),
            Err(e) => {
                tracing::warn!(err = %e, "cannot start the content facts thread");
                self.heavy_stale |= heavy;
            }
        }
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
    store.bypass_change_detection().maybe_start(time.delta(), req);
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
