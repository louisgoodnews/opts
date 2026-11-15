//! Dispatching objects to idle workers and waiting for the run to finish; the shutdown of the pool.

use super::*;

impl Pool {
    pub(in super::super) fn dispatch(&mut self, mut pending: VecDeque<ObjectEntry>) -> Result<()> {
        self.total = pending.len();
        let timeout = Duration::from_secs(self.opts.timeout_s) + Duration::from_secs(15);
        loop {
            for i in 0..self.slots.len() {
                if matches!(self.slots[i].state, State::Idle) {
                    if let Some(entry) = pending.pop_front() {
                        self.send(i, entry);
                    }
                }
            }
            let busy = self
                .slots
                .iter()
                .any(|s| matches!(s.state, State::Busy(..) | State::Starting(_)));
            if pending.is_empty() && !busy {
                return Ok(());
            }
            match self.rx.recv_timeout(Duration::from_millis(200)) {
                Ok((i, generation, ev)) if self.slots[i].generation == generation => {
                    self.handle(i, ev)?
                }
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => bail!("worker channel closed"),
            }
            self.check_timeouts(timeout)?;
        }
    }

    pub(super) fn send(&mut self, i: usize, entry: ObjectEntry) {
        let line = serde_json::to_string(&entry).unwrap_or_default();
        let slot = &mut self.slots[i];
        let ok = slot
            .stdin
            .as_mut()
            .is_some_and(|s| writeln!(s, "{line}").and_then(|()| s.flush()).is_ok());
        // A failed write means the worker died; its EOF event records the crash.
        slot.state = State::Busy(entry, Instant::now());
        if !ok {
            tracing::warn!(worker = i, "cannot write to worker");
        }
    }

    pub(in super::super) fn shutdown(&mut self) {
        for s in &mut self.slots {
            s.stdin = None;
        }
        for s in &mut self.slots {
            let deadline = Instant::now() + Duration::from_secs(10);
            while Instant::now() < deadline {
                if matches!(s.child.try_wait(), Ok(Some(_))) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = s.child.kill();
            let _ = s.child.wait();
        }
    }
}
