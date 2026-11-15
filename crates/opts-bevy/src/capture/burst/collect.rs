//! Frame requests, the collection of encoded frames and the loss bookkeeping of a burst.


use std::time::{Duration, Instant};

use bevy::ecs::world::World;
use bevy::prelude::{Time, Virtual};
use bevy::tasks::{block_on, futures_lite::future, AsyncComputeTaskPool};
use opts_cli::protocol::RpcError;

use crate::control::state::ControlState;
use super::super::screenshot::encode;
use super::super::storage;
use super::state::{Burst, Frame};

/// A requested frame that is not delivered within this time counts as dropped.
const FRAME_WAIT: Duration = Duration::from_secs(2);

impl Burst {
    pub(super) fn request_due(&mut self, world: &mut World, now: f64) -> Result<(), RpcError> {
        if self.frames.len() as u64 >= self.count || now < self.next_due {
            return Ok(());
        }
        let index = self.frames.len() as u64;
        self.next_due = (self.next_due + self.interval).max(now - self.interval);
        let sim_time = world
            .get_resource::<Time<Virtual>>()
            .map_or(0.0, |t| (t.elapsed_secs_f64() * 1000.0).round() / 1000.0);
        let frame = world.get_resource::<ControlState>().map_or(0, |c| c.frame);
        self.frames.push(Frame {
            index,
            sim_time,
            frame,
            wall_ms: ((now - self.started) * 1000.0) as u64,
            requested: Instant::now(),
            path: None,
            resolved: false,
        });
        let tx = self.tx.clone();
        let sink = Box::new(move |image| {
            let _ = tx.send((index, image));
        });
        if let Err(e) = (self.backend.request)(world, sink) {
            if index == 0 {
                return Err(e.into());
            }
            self.mark_lost(index, &e.to_string());
        }
        Ok(())
    }

    pub(super) fn mark_lost(&mut self, index: u64, why: &str) {
        let f = &mut self.frames[index as usize];
        if !f.resolved {
            f.resolved = true;
            self.dropped += 1;
            self.last_error.get_or_insert_with(|| why.to_string());
        }
    }

    pub(super) fn collect(&mut self) {
        while let Ok((index, image)) = self.rx.try_recv() {
            if self.frames[index as usize].resolved {
                continue;
            }
            let path = self
                .dir
                .join(format!("{:04}.{}", index + 1, self.format.ext()));
            self.frames[index as usize].path = Some(path.clone());
            let (format, quality) = (self.format, self.quality);
            let pool = AsyncComputeTaskPool::get_or_init(bevy::tasks::TaskPool::default);
            self.tasks.push((
                index,
                pool.spawn(async move {
                    let bytes = encode(image, format, quality)?;
                    storage::write_atomic(&path, &bytes)
                }),
            ));
        }
        let mut i = 0;
        while i < self.tasks.len() {
            if let Some(result) = block_on(future::poll_once(&mut self.tasks[i].1)) {
                let (index, _) = self.tasks.swap_remove(i);
                let f = &mut self.frames[index as usize];
                f.resolved = true;
                if let Err(e) = result {
                    f.path = None;
                    self.dropped += 1;
                    self.last_error.get_or_insert_with(|| e.to_string());
                }
            } else {
                i += 1;
            }
        }
        let lost: Vec<u64> = self
            .frames
            .iter()
            .filter(|f| !f.resolved && f.path.is_none() && f.requested.elapsed() > FRAME_WAIT)
            .map(|f| f.index)
            .collect();
        for index in lost {
            self.mark_lost(
                index,
                "the renderer did not deliver a frame in time (is the window occluded?)",
            );
        }
    }

    pub(super) fn finished(&self) -> bool {
        self.frames.len() as u64 == self.count && self.frames.iter().all(|f| f.resolved)
    }
}
