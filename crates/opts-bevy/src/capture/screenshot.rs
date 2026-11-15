//! One-shot capture of the primary window: request a frame, encode it off the main thread,
//! write the file and its metadata sidecar.

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task};
use opts_cli::protocol::RpcError;
use serde_json::{Map, Value};

use crate::control::state::{Outcome, Waiter};

use super::error::CaptureError;
use super::metadata::{self, Subject};
use super::storage::{self, CaptureState};

/// A frame that does not arrive within this time fails the command.
pub const FRAME_WAIT: Duration = Duration::from_secs(5);

/// Default JPEG quality.
pub const DEFAULT_JPEG_QUALITY: u8 = 90;

mod finish;
mod frame;
mod options;

pub use finish::*;
pub use frame::*;
pub use options::*;

fn resolve_target(world: &mut World, options: &Options) -> Result<PathBuf, CaptureError> {
    let mut state = world.get_resource_or_insert_with(CaptureState::default);
    let root = state.ensure_root()?;
    match &options.target {
        Target::Generated => {
            let stem = state.next_stem();
            storage::resolve_name(&root, &format!("{stem}.{}", options.format.ext()))
        }
        Target::Name(n) => storage::resolve_name(&root, n),
        Target::Path(p) => storage::resolve_explicit(&root, p),
    }
}

enum Stage {
    Waiting(mpsc::Receiver<Image>, Instant),
    Encoding(Task<Result<Value, CaptureError>>),
}

/// The `screenshot` command.
pub fn handle(world: &mut World, params: &Value) -> Outcome {
    let options = match parse_options(params) {
        Ok(o) => o,
        Err(e) => return Outcome::err(e),
    };
    let path = match resolve_target(world, &options) {
        Ok(p) => p,
        Err(e) => return Outcome::err(e.into()),
    };
    let rx = match request_frame(world) {
        Ok(rx) => rx,
        Err(e) => return Outcome::err(e.into()),
    };
    let meta = options.include_metadata.then(|| {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        metadata::collect(
            world,
            Subject {
                kind: "screenshot",
                file: &file,
                width: 0,
                height: 0,
                format: options.format.name(),
            },
        )
    });
    Outcome::Wait(waiter(rx, path, options, meta))
}

fn waiter(
    rx: mpsc::Receiver<Image>,
    path: PathBuf,
    options: Options,
    mut meta: Option<Map<String, Value>>,
) -> Waiter {
    let mut stage = Some(Stage::Waiting(rx, Instant::now()));
    Box::new(move |_| match stage.take()? {
        Stage::Waiting(rx, began) => match rx.try_recv() {
            Ok(image) => {
                let (path, options, meta) = (path.clone(), options.clone(), meta.take());
                let pool = AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
                stage = Some(Stage::Encoding(
                    pool.spawn(async move { finish(image, &path, &options, meta) }),
                ));
                None
            }
            Err(mpsc::TryRecvError::Empty) if began.elapsed() < FRAME_WAIT => {
                stage = Some(Stage::Waiting(rx, began));
                None
            }
            Err(_) => Some(Err(CaptureError::Timeout(
                "the frame did not arrive in time (is the window occluded?)".into(),
            )
            .into())),
        },
        Stage::Encoding(mut task) => match block_on(future::poll_once(&mut task)) {
            Some(result) => Some(result.map_err(RpcError::from)),
            None => {
                stage = Some(Stage::Encoding(task));
                None
            }
        },
    })
}

#[cfg(test)]
mod tests;
