//! Starting a worker process: its directories, its pipes and the reader thread of its messages.

use super::*;

impl Pool {
    pub(in super::super) fn new(run_dir: &Path, opts: &RunOptions) -> Result<Self> {
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

    pub(super) fn worker_dir(&self, i: usize) -> PathBuf {
        self.run_dir.join(format!("work/worker-{i}"))
    }

    pub(super) fn spawn(&self, i: usize, generation: u64) -> Result<Slot> {
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
}
