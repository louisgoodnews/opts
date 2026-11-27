//! The coordinator: queues tile parse jobs, runs at most `max_in_flight` of them on the async
//! compute pool and keeps the finished results until the streamer takes them.
//!
//! Work happens in [`TileParseCoordinator::poll`], which the frame calls. A poll first collects the
//! jobs that finished since the last poll, then starts queued jobs while there are free slots.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task, TaskPool};

use super::parse::{parse_tile_file, TileParseResult};

/// A tile's key `(x, y)` as in `tile_<x>_<y>.map`.
pub type TileKey = (i32, i32);

/// Queues, runs and collects tile parse jobs, one job per tile key.
pub struct TileParseCoordinator {
    max_in_flight: usize,
    queued: VecDeque<(TileKey, PathBuf)>,
    running: HashMap<TileKey, Task<TileParseResult>>,
    finished: HashMap<TileKey, TileParseResult>,
}

impl TileParseCoordinator {
    /// A coordinator that runs at most `max_in_flight` jobs at once (at least one).
    #[must_use]
    pub fn new(max_in_flight: usize) -> Self {
        Self {
            max_in_flight: max_in_flight.max(1),
            queued: VecDeque::new(),
            running: HashMap::new(),
            finished: HashMap::new(),
        }
    }

    /// Queues a parse of `path` for `key`. Returns `false` (and queues nothing) when the key is
    /// already queued, running or finished and not yet taken.
    pub fn request(&mut self, key: TileKey, path: PathBuf) -> bool {
        if self.is_known(key) {
            return false;
        }
        self.queued.push_back((key, path));
        true
    }

    /// Collects the jobs that finished, then starts queued jobs up to the in-flight limit.
    pub fn poll(&mut self) {
        self.harvest();
        self.start_queued();
    }

    /// Takes the finished result of `key`, if there is one. A result is handed out once.
    pub fn take(&mut self, key: TileKey) -> Option<TileParseResult> {
        self.finished.remove(&key)
    }

    /// Drops every trace of `key`: a queued request, a running job (dropping the task cancels it)
    /// and a finished result. Used when the tile is no longer wanted.
    pub fn cancel(&mut self, key: TileKey) {
        self.queued.retain(|(k, _)| *k != key);
        self.running.remove(&key);
        self.finished.remove(&key);
    }

    /// Whether `key` is queued, running or finished and not yet taken.
    #[must_use]
    pub fn is_known(&self, key: TileKey) -> bool {
        self.queued.iter().any(|(k, _)| *k == key)
            || self.running.contains_key(&key)
            || self.finished.contains_key(&key)
    }

    /// Jobs waiting for a free slot.
    #[must_use]
    pub fn queued_count(&self) -> usize {
        self.queued.len()
    }

    /// Jobs running on the async pool.
    #[must_use]
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// The in-flight limit.
    #[must_use]
    pub fn max_in_flight(&self) -> usize {
        self.max_in_flight
    }

    fn harvest(&mut self) {
        let mut ready = Vec::new();
        for (key, task) in &mut self.running {
            if let Some(result) = block_on(future::poll_once(task)) {
                ready.push((*key, result));
            }
        }
        for (key, result) in ready {
            self.running.remove(&key);
            self.finished.insert(key, result);
        }
    }

    fn start_queued(&mut self) {
        if self.queued.is_empty() {
            return;
        }
        let pool = AsyncComputeTaskPool::get_or_init(TaskPool::default);
        while self.running.len() < self.max_in_flight {
            let Some((key, path)) = self.queued.pop_front() else {
                break;
            };
            let task = pool.spawn(async move { parse_tile_file(&path) });
            self.running.insert(key, task);
        }
    }
}
