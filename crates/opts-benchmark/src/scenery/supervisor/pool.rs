//! Worker process pool: spawns, dispatches objects over stdin, handles crashes/timeouts, and
//! recycles workers.

use super::*;

enum Event {
    Msg(WorkerMsg),
    Eof,
}

enum State {
    Starting(Instant),
    Idle,
    Busy(ObjectEntry, Instant),
    Leaving,
}

struct Slot {
    child: Child,
    stdin: Option<ChildStdin>,
    state: State,
    generation: u64,
    failed_starts: usize,
}

pub(super) struct Pool {
    run_dir: PathBuf,
    opts: RunOptions,
    slots: Vec<Slot>,
    tx: Sender<(usize, u64, Event)>,
    rx: Receiver<(usize, u64, Event)>,
    results: ResultsWriter,
    finished: usize,
    total: usize,
    started: Instant,
}

impl Pool {
    pub(super) fn new(run_dir: &Path, opts: &RunOptions) -> Result<Self> {
        let (tx, rx) = channel();
        let mut pool = Self {
            run_dir: run_dir.to_path_buf(),
            opts: opts.clone(),
            slots: Vec::new(),
            tx,
            rx,
            results: ResultsWriter::open(&run_dir.join("results.jsonl"))?,
            finished: 0,
            total: 0,
            started: Instant::now(),
        };
        for i in 0..opts.workers.max(1) {
            let slot = pool.spawn(i, 0)?;
            pool.slots.push(slot);
        }
        Ok(pool)
    }

    fn worker_dir(&self, i: usize) -> PathBuf {
        self.run_dir.join(format!("work/worker-{i}"))
    }

    fn spawn(&self, i: usize, generation: u64) -> Result<Slot> {
        let dir = self.worker_dir(i);
        std::fs::create_dir_all(dir.join("config"))?;
        std::fs::create_dir_all(dir.join("data"))?;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("worker.log"))?;
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.args(["scenery", "worker", "--run-dir"])
            .arg(&self.run_dir)
            .args(["--worker-id", &i.to_string()])
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_DATA_HOME", dir.join("data"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(log));
        if let Some(c) = &self.opts.cache_dir {
            cmd.env("XDG_CACHE_HOME", c);
        }
        let mut child = cmd.spawn().context("start worker")?;
        let stdout = child.stdout.take().context("worker stdout")?;
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(m) = WorkerMsg::from_line(&line) {
                    if tx.send((i, generation, Event::Msg(m))).is_err() {
                        return;
                    }
                }
            }
            let _ = tx.send((i, generation, Event::Eof));
        });
        Ok(Slot {
            stdin: child.stdin.take(),
            child,
            state: State::Starting(Instant::now()),
            generation,
            failed_starts: 0,
        })
    }

    pub(super) fn dispatch(&mut self, mut pending: VecDeque<ObjectEntry>) -> Result<()> {
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

    fn send(&mut self, i: usize, entry: ObjectEntry) {
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

    fn handle(&mut self, i: usize, ev: Event) -> Result<()> {
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

    fn record(&mut self, r: &Record) -> Result<()> {
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
    fn lost(&mut self, i: usize, category: &str) -> Result<()> {
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

    fn check_timeouts(&mut self, object_timeout: Duration) -> Result<()> {
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

    pub(super) fn shutdown(&mut self) {
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

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        std::fs::copy(e.path(), to.join(e.file_name()))?;
    }
    Ok(())
}
