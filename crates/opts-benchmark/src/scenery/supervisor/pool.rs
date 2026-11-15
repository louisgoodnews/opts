//! Worker process pool: spawns, dispatches objects over stdin, handles crashes/timeouts, and
//! recycles workers.

use super::*;

mod dispatch;
mod events;
mod spawn;

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
