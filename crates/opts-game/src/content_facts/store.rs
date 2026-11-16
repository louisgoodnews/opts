//! The cached facts and the request in flight: the store the Content page reads, and the
//! worker threads that gather the facts.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::prelude::*;

use super::disk::DiskFacts;
use super::{ContentFacts, FactsProvider, HeavyFacts, LightFacts, LIGHT_REFRESH};

/// What a worker sends back: the light facts and, when asked, the heavy ones with the theme list.
pub(super) type Answer = (LightFacts, Option<(HeavyFacts, Vec<String>)>);

/// The cached facts and the request in flight.
#[derive(Resource)]
pub struct ContentFactsStore {
    pub(super) provider: Arc<dyn FactsProvider>,
    pub(super) current: Option<ContentFacts>,
    pub(super) light_ready: bool,
    pub(super) heavy_ready: bool,
    pub(super) heavy_stale: bool,
    pub(super) pending: Option<(Mutex<Receiver<Answer>>, bool)>,
    pub(super) jobs: Vec<Mutex<Receiver<()>>>,
    pub(super) themes: Vec<String>,
    pub(super) since_light: Duration,
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
        self.light_ready
            .then(|| self.current.unwrap_or_default().light)
    }

    /// The heavy facts, once known (a stale value stays visible until the new one arrives).
    #[must_use]
    pub fn heavy(&self) -> Option<HeavyFacts> {
        self.heavy_ready
            .then(|| self.current.unwrap_or_default().heavy)
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
}
