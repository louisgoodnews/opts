//! Handling worker messages and exits: results, lost workers, timeouts and the copy of a repro.

use super::*;

impl Pool {
    pub(super) fn handle(&mut self, i: usize, ev: Event) -> Result<()> {
        match ev {
            Event::Msg(WorkerMsg::Ready) => {
                self.slots[i].state = State::Idle;
                self.slots[i].failed_starts = 0;
            }
            Event::Msg(WorkerMsg::Start { .. }) => {}
            Event::Msg(WorkerMsg::Result { record }) => {
                self.record(&record)?;
                if matches!(self.slots[i].state, State::Busy(..)) {
                    self.slots[i].state = State::Idle;
                }
            }
            Event::Msg(WorkerMsg::Recycle { reason }) => {
                tracing::info!(worker = i, %reason, "worker recycles");
                self.slots[i].state = State::Leaving;
            }
            Event::Eof => self.lost(i, "F-CRASH")?,
        }
        Ok(())
    }

    pub(super) fn record(&mut self, r: &Record) -> Result<()> {
        self.results.append(r)?;
        self.finished += 1;
        if self.finished.is_multiple_of(25) || self.finished == self.total {
            let rate = self.started.elapsed().as_secs_f64() / self.finished as f64;
            tracing::info!(
                done = self.finished,
                total = self.total,
                s_per_object = format!("{rate:.3}"),
                "progress"
            );
        }
        Ok(())
    }

    /// A worker is gone (exit, crash or kill): record its object, start a successor.
    pub(super) fn lost(&mut self, i: usize, category: &str) -> Result<()> {
        let _ = self.slots[i].child.kill();
        let _ = self.slots[i].child.wait();
        let state = std::mem::replace(&mut self.slots[i].state, State::Leaving);
        let failed_starts = match state {
            State::Busy(entry, since) => {
                tracing::warn!(worker = i, sco = %entry.sco, category, "object lost");
                let mut r =
                    Record::aborted(&entry, category, i, since.elapsed().as_millis() as u64);
                let dest = self.run_dir.join("repro").join(&entry.key);
                let map = self.worker_dir(i).join("map");
                if copy_dir(&map, &dest).is_ok() {
                    r.repro = Some(format!("XDG_CONFIG_HOME=$(mktemp -d) opts-game --autostart --map-path {} --omsi-path {} --camera free", dest.display(), self.opts.omsi_root.display()));
                }
                self.record(&r)?;
                0
            }
            State::Starting(_) => self.slots[i].failed_starts + 1,
            _ => 0,
        };
        if failed_starts >= 3 {
            bail!(
                "worker {i} failed to start three times; see {}",
                self.worker_dir(i).join("worker.log").display()
            );
        }
        let generation = self.slots[i].generation + 1;
        let mut slot = self.spawn(i, generation)?;
        slot.failed_starts = failed_starts;
        self.slots[i] = slot;
        Ok(())
    }

    pub(super) fn check_timeouts(&mut self, object_timeout: Duration) -> Result<()> {
        for i in 0..self.slots.len() {
            let expired = match &self.slots[i].state {
                State::Busy(_, since) => since.elapsed() > object_timeout,
                State::Starting(since) => since.elapsed() > Duration::from_secs(180),
                _ => false,
            };
            if expired {
                let cat = if matches!(self.slots[i].state, State::Busy(..)) {
                    "F-TIMEOUT"
                } else {
                    "F-CRASH"
                };
                self.lost(i, cat)?;
            }
        }
        Ok(())
    }
}

pub(super) fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        std::fs::copy(e.path(), to.join(e.file_name()))?;
    }
    Ok(())
}
