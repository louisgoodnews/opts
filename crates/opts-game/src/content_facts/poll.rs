//! Delivery of the finished answers and the start of due requests on [`ContentFactsStore`].

use std::sync::mpsc::{self, TryRecvError};
use std::sync::Mutex;
use std::time::Duration;

use super::store::ContentFactsStore;
use super::{FactsRequest, LIGHT_REFRESH};

impl ContentFactsStore {
    /// Takes a finished answer, if any. Never blocks.
    pub(super) fn poll(&mut self) {
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
        let received = rx
            .lock()
            .map_or(Err(TryRecvError::Disconnected), |rx| rx.try_recv());
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
    pub(super) fn maybe_start(&mut self, elapsed: Duration, req: FactsRequest) {
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
